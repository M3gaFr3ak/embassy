//! DMA support: the ring-buffered regular-conversion filter that reads
//! converted samples via DMA.

use super::*;
use crate::dma::ringbuffer::Error as DmaError;
use crate::dma::{Channel, ReadableRingBuffer};
use crate::interrupt::typelevel::Binding;
use crate::rcc::WakeGuard;

// =============================================================================
// DMA ring buffer read path
// =============================================================================

/// A filter bound to a DMA ring buffer, for reading converted samples.
pub struct RingBufferedFilter<'e, T, M, DM: DmaMode>
where
    T: Instance + FilterInterrupt<M>,
    M: FilterMarker + InstanceEvents<T>,
{
    _dma_marker: PhantomData<DM>,
    filter: &'e mut dyn FilterDma<T, M>,
    ring_buf: ReadableRingBuffer<'e, u32>,
    _wake_guard: WakeGuard,
}

impl<'a, 'd, 't, T, M> FilterRegular<'a, 'd, 't, T, M, RegDma>
where
    T: Instance + FilterInterrupt<M>,
    M: FilterMarker + InstanceEvents<T>,
{
    /// Attach a DMA ring buffer to this filter's regular-conversion data register.
    pub fn ring_buffered<'e, D: Dma<T, M>>(
        &'e mut self,
        dma: Peri<'e, D>,
        irq: impl Binding<D::Interrupt, crate::dma::InterruptHandler<D>> + 'e,
        dma_buf: &'e mut [u32],
    ) -> RingBufferedFilter<'e, T, M, RegDma> {
        let mut buf = RingBufferedFilter::<T, M, RegDma>::new_int(self, dma, irq, dma_buf);
        buf.ring_buf.set_alignment(1);
        buf
    }
}

impl<'a, 'd, 't, T, M> FilterInjected<'a, 'd, 't, T, M, InjDma>
where
    T: Instance + FilterInterrupt<M>,
    M: FilterMarker + InstanceEvents<T>,
{
    /// Attach a DMA ring buffer to this filter's injected-conversion data register.
    pub fn ring_buffered<'e, D: Dma<T, M>>(
        &'e mut self,
        dma: Peri<'e, D>,
        irq: impl Binding<D::Interrupt, crate::dma::InterruptHandler<D>> + 'e,
        dma_buf: &'e mut [u32],
    ) -> RingBufferedFilter<'e, T, M, InjDma> {
        let alignment = self.popcnt();
        let mut buf = RingBufferedFilter::<T, M, InjDma>::new_int(self, dma, irq, dma_buf);
        buf.ring_buf.set_alignment(alignment);
        buf
    }

    /// Returns number of assigned transceivers in the injected group.
    fn popcnt(&self) -> usize {
        let bitmask = T::regs().flt(M::CHANNEL.index()).jchgr().read().jchg();
        bitmask.count_ones() as usize
    }
}

#[allow(private_bounds)]
impl<'e, T, M, DM: DmaMode> RingBufferedFilter<'e, T, M, DM>
where
    T: Instance + FilterInterrupt<M>,
    M: FilterMarker + InstanceEvents<T>,
{
    fn new_int<D: Dma<T, M>, DMODE: DmaMode>(
        filter: &'e mut dyn FilterDma<T, M>,
        dma: Peri<'e, D>,
        irq: impl Binding<D::Interrupt, crate::dma::InterruptHandler<D>> + 'e,
        dma_buf: &'e mut [u32],
    ) -> RingBufferedFilter<'e, T, M, DMODE> {
        let opts = Default::default();

        // SAFETY: `ReadableRingBuffer::new` requires:
        // 1. Exclusive access to the DMA channel - guaranteed by taking ownership of `dma: Peri<'e, D>`
        // 2. The buffer pointer remains valid for the lifetime `'e` - guaranteed by the borrow `dma_buf: &'e mut [u32]`
        // 3. The data register pointer is valid - `filter.data_register()` returns a valid MMIO address
        // 4. No concurrent access to the buffer - the ring buffer is not started until `start()` is called
        // SAFETY: `filter.data_register()` returns a pointer to the DFSDM filter's
        // data register (RDATAR or JDATAR), which is a valid DMA source. The register
        // is memory-mapped and remains accessible for the lifetime of the filter.
        let request = dma.request();
        let ring_buf =
            unsafe { ReadableRingBuffer::new(Channel::new(dma, irq), request, filter.data_register(), dma_buf, opts) };

        RingBufferedFilter {
            _dma_marker: PhantomData,
            filter,
            _wake_guard: T::RCC_INFO.wake_guard(),
            ring_buf,
        }
    }

    /// Start a conversion. Regular conversions start one conversion (or
    /// continuous, if enabled); injected conversions start one group (or a
    /// scan, if enabled).
    pub fn start_conversion(&mut self) {
        self.filter.start_conversion();
    }

    /// Start the DMA transfers. Does not start a conversion; call
    /// [`start_conversion`](Self::start_conversion) separately.
    pub fn start(&mut self) {
        self.ring_buf.start();
    }

    /// Pause the DMA transfers. Conversions are not stopped.
    pub fn stop(&mut self) {
        self.ring_buf.request_pause();
    }

    /// Discard all buffered samples.
    pub fn clear(&mut self) {
        self.ring_buf.clear();
    }

    /// Whether the DMA ring buffer is running.
    pub fn is_running(&mut self) -> bool {
        self.ring_buf.is_running()
    }

    /// Number of samples the ring buffer can hold.
    pub fn capacity(&self) -> usize {
        self.ring_buf.capacity()
    }

    /// Read the most recent samples, discarding older data. Never blocks;
    /// returns the number of samples written into `buf`.
    pub fn read_latest(&mut self, buf: &mut [u32]) -> Result<usize, Error> {
        self.autostart()?;

        Ok(self.ring_buf.read_latest(buf))
    }

    /// Asynchronously read `buf.len()` samples. `buf.len()` must equal half of
    /// [`capacity`](Self::capacity), or this panics. Starts the DMA if needed;
    /// returns [`Error::Overrun`] if the buffer overran.
    pub async fn read(&mut self, buf: &mut [u32]) -> Result<usize, Error> {
        assert_eq!(
            self.ring_buf.capacity() / 2,
            buf.len(),
            "Buffer size must be half the size of the ring buffer"
        );

        self.autostart()?;

        self.ring_buf.read_exact(buf).await.map_err(remap_dma_error)
    }

    /// Blocking counterpart of [`read`](Self::read): spins until `buf.len()`
    /// samples are available, with the same half-capacity requirement.
    pub fn blocking_read(&mut self, buf: &mut [u32]) -> Result<usize, Error> {
        assert_eq!(
            self.ring_buf.capacity() / 2,
            buf.len(),
            "Buffer size must be half the size of the ring buffer"
        );

        self.autostart()?;

        loop {
            match self.ring_buf.read(buf) {
                Ok((0, _)) => {}
                Ok((len, _)) => {
                    return Ok(len);
                }
                Err(err) => {
                    self.ring_buf.request_pause();

                    return Err(remap_dma_error(err));
                }
            }
        }
    }

    fn autostart(&mut self) -> Result<(), Error> {
        if self.filter.get_and_clear_overrun() {
            return Err(Error::Overrun);
        }

        if !self.ring_buf.is_running() {
            self.start();
        }

        Ok(())
    }
}

fn remap_dma_error(err: DmaError) -> Error {
    match err {
        DmaError::Overrun => Error::Overrun,
        DmaError::DmaUnsynced => Error::PeripheralError,
    }
}
