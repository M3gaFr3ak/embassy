//! Transceiver driver, its pin reference-counting storage, single-use pin
//! selectors, and the pin-trait associations.

use super::*;

// =============================================================================
// Pin Reference Counting Storage
// =============================================================================

/// Which of a transceiver's two pins a slot tracks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PinKind {
    /// Data-input pin.
    Datin,
    /// Clock-input pin.
    Ckin,
}

/// Reference-counted storage for one pin of one transceiver.
pub struct PinSlot<'d> {
    pub(crate) inner: critical_section::Mutex<RefCell<Option<Flex<'d>>>>,
    pub(crate) rc: AtomicU8,
}

impl<'d> PinSlot<'d> {
    pub(crate) const fn new() -> Self {
        Self {
            inner: critical_section::Mutex::new(RefCell::new(None)),
            rc: AtomicU8::new(0),
        }
    }
}

// =============================================================================
// Transceiver
// =============================================================================

/// Configured DFSDM data input transceiver.
pub struct Transceiver<'a, 'd, T, M, S, MODE, PS, P>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    PS: PinSource,
    P: PowerState,
{
    pub(crate) common: &'a DfsdmCommon<'d, T, Enabled>,
    _instance_marker: PhantomData<T>,
    _transceiver_marker: PhantomData<M>,
    _pinset_marker: PhantomData<S>,
    _channel_mode_marker: PhantomData<MODE>,
    _pin_source_marker: PhantomData<PS>,
    _powerstate_marker: PhantomData<P>,
}

impl<'a, 'd, T, M, S, MODE, PS, P> Transceiver<'a, 'd, T, M, S, MODE, PS, P>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    PS: PinSource,
    P: PowerState,
{
    fn new(common: &'a DfsdmCommon<'d, T, Enabled>) -> Self {
        Self {
            common,
            _instance_marker: PhantomData,
            _transceiver_marker: PhantomData,
            _pinset_marker: PhantomData,
            _channel_mode_marker: PhantomData,
            _pin_source_marker: PhantomData,
            _powerstate_marker: PhantomData,
        }
    }
}

impl<'a, 'd, T, M, S, MODE, PS, P> Drop for Transceiver<'a, 'd, T, M, S, MODE, PS, P>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    PS: PinSource,
    P: PowerState,
{
    fn drop(&mut self) {
        // "Drop pin references" as we "manually" reference count
        let ch = if PS::FROM_NEIGHBOR {
            <M::Next as TransceiverMarker>::CHANNEL.index()
        } else {
            M::CHANNEL.index()
        };

        if S::HAS_DATA {
            self.common.release_pin(ch, PinKind::Datin);
        }
        if S::HAS_CLK {
            self.common.release_pin(ch, PinKind::Ckin);
        }

        // Disabling will deactivate the detector flags,
        // so we need to remove them from the cached version
        ShortCircuitDetector::<T>::drop_transceiver(M::CHANNEL);
        ClockAbsenceDetector::<T>::drop_transceiver(M::CHANNEL);

        Self::set_enabled(false);
    }
}

/// Only when enabled
impl<'a, 'd, T, M, S, MODE, PS> Transceiver<'a, 'd, T, M, S, MODE, PS, Enabled>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    PS: PinSource,
{
    /// Disables the transceiver.
    pub fn disable(self) -> Transceiver<'a, 'd, T, M, S, MODE, PS, Disabled> {
        Self::set_enabled(false);

        let common = self.common;
        core::mem::forget(self);
        Transceiver::new(common)
    }
}

impl<'a, 'd, T, M, S, MODE, PS> Transceiver<'a, 'd, T, M, S, MODE, PS, Enabled>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode + ExternalSerialMode,
    PS: PinSource,
{
    /// Wait until this transceiver's clock-absence flag clears, indicating it
    /// is synchronized. Only meaningful for externally-clocked serial modes.
    pub async fn wait_for_sync(&mut self) {
        loop {
            if ClockAbsenceDetector::<T>::try_clear_channel_flag(M::CHANNEL) {
                break;
            }
            #[cfg(feature = "time")]
            embassy_time::Timer::after_millis(1).await;

            #[cfg(not(feature = "time"))]
            {
                let freq = unsafe { crate::rcc::get_freqs() }.sys.to_hertz().unwrap().0 as u64;
                let cycles = freq / 1_000; // 1ms
                cortex_m::asm::delay(cycles as u32);
            }
        }
    }
}

/// Only when disabled
impl<'a, 'd, T, M, S, MODE, PS> Transceiver<'a, 'd, T, M, S, MODE, PS, Disabled>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    PS: PinSource,
{
    /// Enables the transceiver.
    pub fn enable(self) -> Transceiver<'a, 'd, T, M, S, MODE, PS, Enabled> {
        Self::set_enabled(true);

        let common = self.common;
        core::mem::forget(self);

        Transceiver::new(common)
    }

    /// Set the transceiver's right-shift factor.
    pub fn set_data_right_shift(self, shift: config::DataRightShift) -> Self {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr2()
            .modify(|w| w.set_dtrbs(shift.into()));
        self
    }

    /// Set the analog watchdog filter's order.
    pub fn select_awd_filter_order(self, filter_order: config::AwdFilterOrder) -> Self {
        T::regs()
            .ch(M::CHANNEL.index())
            .awscdr()
            .modify(|w| w.set_awford(filter_order as u8));
        self
    }

    /// Set the analog watchdog filter's OSR.
    pub fn select_awd_filter_osr(self, osr: config::AwdFilterOsr) -> Self {
        T::regs()
            .ch(M::CHANNEL.index())
            .awscdr()
            .modify(|w| w.set_awfosr(osr.into()));
        self
    }
}

impl<'a, 'd, T, M, S, PS, P> Transceiver<'a, 'd, T, M, S, ParallelDmaMode, PS, P>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    P: PowerState,
    PS: PinSource,
{
    /// Get direct pointer to the DATINR register for DMA mem2mem use
    pub fn get_datinr_as_ptr(&self) -> *mut u32 {
        T::regs().ch(M::CHANNEL.index()).datinr().as_ptr() as *mut u32
    }

    /// Manually write one sample into the DATINR register, used for standard mode
    pub fn write_sample_standard(&self, data: u16) {
        T::regs().ch(M::CHANNEL.index()).datinr().write(|w| w.set_indat0(data));
    }

    /// Manually write two subsequent samples into the DATINR register, used for interleaved mode
    pub fn write_indat1(&self, data: [u16; 2]) {
        T::regs().ch(M::CHANNEL.index()).datinr().write(|w| {
            w.set_indat0(data[0]);
            w.set_indat1(data[1]);
        });
    }
}
/// Any powerstate
impl<'a, 'd, T, M, S, MODE, PS, P> Transceiver<'a, 'd, T, M, S, MODE, PS, P>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode,
    P: PowerState,
    PS: PinSource,
{
    /// Enable/Disable the transceiver
    pub(crate) fn set_enabled(enabled: bool) {
        T::regs().ch(M::CHANNEL.index()).cfgr1().modify(|w| w.set_chen(enabled));
    }

    /// Set the transceiver's offset.
    pub fn set_offset(&mut self, offset: u32) {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr2()
            .modify(|w| w.set_offset(offset));
    }

    /// Read the analog watchdog data for this transceiver, converted by the
    /// watchdog filter (continuously, with limited resolution).
    pub fn awd_filter_data(&self) -> u16 {
        T::regs().ch(M::CHANNEL.index()).wdatr().read().wdata()
    }
}

impl<'a, 'd, T, M, S, MODE, PS, P> Transceiver<'a, 'd, T, M, S, MODE, PS, P>
where
    T: Instance + HasDelay,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    MODE: ChannelMode + SerialMode,
    P: PowerState,
    PS: PinSource,
{
    /// Reads back pulses still to skip, 0 = done
    pub fn skip_progress(&self) -> u8 {
        T::regs().ch(M::CHANNEL.index()).dlyr().read().plsskp()
    }

    /// Configure to skip the next `skips` pulses (max 63 per write).
    ///
    /// Skipping starts immediately on write; updating mid-skip is allowed.
    /// To skip more than 63 pulses, issue repeated writes; the peripheral
    /// doesn't track a cumulative count across writes, so the caller must.
    pub fn skip_pulses(&mut self, skips: config::PulsesToSkip) {
        self.set_pulseskips(skips);
    }

    /// Set pulse `skips`
    fn set_pulseskips(&mut self, skips: config::PulsesToSkip) {
        T::regs()
            .ch(M::CHANNEL.index())
            .dlyr()
            .modify(|w| w.set_plsskp(skips.into()));
    }
}

/// The two transceivers returned by
/// [`TransceiverBuilder::build_parallel_dma_dual`]: this channel (`M`/`S`, owns
/// the DATINR register) and its paired successor (`MN`/`SN`, reads INDAT1).
pub type ParallelDmaPair<'a, 'd, T, M, S, MN, SN> = (
    Transceiver<'a, 'd, T, M, S, ParallelDmaMode, OwnPins, Disabled>,
    Transceiver<'a, 'd, T, MN, SN, ParallelDmaMode, OwnPins, Disabled>,
);

/// Used to build a [`Transceiver`].
pub struct TransceiverBuilder<T, M, C, S, SN>
where
    T: Instance,
    M: TransceiverMarker,
    C: ClockOutputMode,
    S: PinSet,  //Own pins
    SN: PinSet, //Neighbors pins
{
    _m: PhantomData<(T, M, C, S, SN)>,
}

impl<T, M, C, S, SN> TransceiverBuilder<T, M, C, S, SN>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    C: ClockOutputMode,
    S: PinSet,
    SN: PinSet,
{
    /// Creates a new builder for a transceiver.
    pub(crate) fn new() -> Self {
        Self { _m: PhantomData }
    }

    /// Parallel input from ADC writes to CHyDATINR (DATMPX=1).
    /// No CKOUT, no pins needed. Serial pins declared on this transceiver
    /// are disconnected (the builder's Flexes drop here - they're unused
    /// in this mode).
    pub fn build_parallel_adc<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
    ) -> Transceiver<'a, 'd, T, M, S, ParallelAdcMode, OwnPins, Disabled>
    where
        T: capability::AdcInput,
    {
        self.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::InternalAdc);
        Transceiver::new(common)
    }

    /// Parallel input from CPU/DMA writes to CHyDATINR (DATMPX=2).
    /// No CKOUT, no pins needed. Serial pins declared on this transceiver
    /// are disconnected (the builder's Flexes drop here - they're unused
    /// in this mode).
    pub fn build_parallel_dma<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        packing_mode: config::DataPackingModeReduced,
    ) -> Transceiver<'a, 'd, T, M, S, ParallelDmaMode, OwnPins, Disabled> {
        self.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::InternalRegisterWrite);
        self.set_data_packing_mode(packing_mode.into());
        Transceiver::new(common)
    }

    /// Create a dual-mode DMA pair.
    ///
    /// Returns transceivers for transceiver `M` (even, owns DATINR) and `MN`
    /// (odd, reads INDAT1 from M's DATINR). Two filters must be configured -
    /// one assigned to `M` (reads INDAT0, the lower word) and one to `MN`
    /// (reads INDAT1, the upper word) - or the register won't drain and
    /// you'll get overrun errors.
    pub fn build_parallel_dma_dual<'a, 'd, MN, SNN>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mut neighbor: TransceiverBuilder<T, MN, C, SN, SNN>,
    ) -> ParallelDmaPair<'a, 'd, T, M, S, MN, SN>
    where
        M: DualPackingAllowed + NextChannelForInstance<T, Next = MN>,
        MN: TransceiverMarker + NextChannelForInstance<T>,
        SNN: PinSet,
    {
        self.select_channel_input(config::ChannelInput::Same);
        neighbor.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::InternalRegisterWrite);
        neighbor.select_data_mux_input(config::InputDataMux::InternalRegisterWrite);
        self.set_data_packing_mode(config::DataPackingMode::Dual);
        neighbor.set_data_packing_mode(config::DataPackingMode::Standard);
        (Transceiver::new(common), Transceiver::new(common))
    }

    /// Manchester-coded input over this transceiver's own DATIN pin (SITP = 2/3,
    /// DATMPX = 0). The clock is recovered from the data line, so CKOUT/CKIN
    /// are not needed; the declared DATIN pin carries data *and* clock.
    /// `mode` chooses the Manchester polarity (rising edge = 0 or 1).
    pub fn build_manchester<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::ManchesterMode,
    ) -> Transceiver<'a, 'd, T, M, S, ManchesterMode, OwnPins, Disabled>
    where
        S: HasData,
    {
        self.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        Transceiver::new(common)
    }

    /// Same as [`Self::build_manchester`], but using the neighbor's pins.
    pub fn build_manchester_neighbor<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::ManchesterMode,
    ) -> Result<Transceiver<'a, 'd, T, M, DataOnly, ManchesterMode, NeighborPins, Disabled>, Error>
    where
        SN: HasData,
    {
        let next_ch = <M::Next as TransceiverMarker>::CHANNEL.index();
        common.acquire_pin(next_ch, PinKind::Datin)?;

        self.select_channel_input(config::ChannelInput::Neighbor);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        Ok(Transceiver::new(common))
    }

    /// SPI input over this transceiver's own pins (DATMPX=0, SPICKSEL=0): sampling
    /// clock comes from the *external* CKIN pin; requires a `DataClk` pinset
    /// (both lines). `mode` chooses rising/falling-edge sampling (SITP 0/1).
    pub fn build_spi_ext<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::SpiMode,
    ) -> Transceiver<'a, 'd, T, M, S, SpiExtMode, OwnPins, Disabled>
    where
        S: HasDataAndClk,
    {
        self.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        self.select_spi_clock(config::SpiClockSelect::ExternalCkin);
        Transceiver::new(common)
    }

    /// Same as [`Self::build_spi_ext`], but using the neighbor's pins.
    pub fn build_spi_ext_neighbor<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::SpiMode,
    ) -> Result<Transceiver<'a, 'd, T, M, DataClk, SpiExtMode, NeighborPins, Disabled>, Error>
    where
        SN: HasDataAndClk,
    {
        let next_ch = <M::Next as TransceiverMarker>::CHANNEL.index();
        common.acquire_pins::<DataClk>(next_ch)?;

        self.select_channel_input(config::ChannelInput::Neighbor);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        self.select_spi_clock(config::SpiClockSelect::ExternalCkin);
        Ok(Transceiver::new(common))
    }

    fn set_data_packing_mode(&mut self, mode: config::DataPackingMode) {
        // Dual mode is
        // available only on even channel numbers (y = 0, 2, 4, 6), for odd channel numbers (y = 1, 3, 5, 7)
        // DFSDM_CHyDATINR is write protected. If an even channel is set to dual mode then the following
        // odd channel must be set into standard mode (DATPACK[1:0]=0) for correct cooperation with even
        // channel.
        //  could make that explicit with a semantic constructor:
        // ch0.new_parallel_dma_dual()
        // meaning:
        // "ch0 and its paired successor are now configured as a dual-input pair."
        // then keeping the odd one for yourself, idk

        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr1()
            .modify(|w| w.set_datpack(mode as u8));
    }

    fn select_data_mux_input(&mut self, input: config::InputDataMux) {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr1()
            .modify(|w| w.set_datmpx(input as u8));
    }

    fn select_channel_input(&mut self, source: config::ChannelInput) {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr1()
            .modify(|w| w.set_chinsel(source.into()));
    }

    fn select_spi_clock(&mut self, source: config::SpiClockSelect) {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr1()
            .modify(|w| w.set_spicksel(source as u8));
    }

    fn select_serial_interface_type(&mut self, if_type: config::SerialInterfaceType) {
        T::regs()
            .ch(M::CHANNEL.index())
            .cfgr1()
            .modify(|w| w.set_sitp(if_type as u8));
    }
}

impl<T, M, S, SN> TransceiverBuilder<T, M, OutputEnabled, S, SN>
where
    T: Instance,
    M: TransceiverMarker + NextChannelForInstance<T>,
    S: PinSet,
    SN: PinSet,
{
    /// SPI input over this transceiver's own DATIN pin (DATMPX=0), clock supplied
    /// by our own CKOUT - only meaningful with `OutputEnabled`
    /// (`InternalSpiMode` picks rising/falling or the half-rate edges).
    pub fn build_spi_int<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::InternalSpiMode,
    ) -> Transceiver<'a, 'd, T, M, S, SpiCkoutMode, OwnPins, Disabled>
    where
        S: HasData,
    {
        self.select_channel_input(config::ChannelInput::Same);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        self.select_spi_clock(mode.into());
        Transceiver::new(common)
    }

    /// Same as [`Self::build_spi_int`], but using the neighbor's pins.
    pub fn build_spi_int_neighbor<'a, 'd>(
        mut self,
        common: &'a DfsdmCommon<'d, T, Enabled>,
        mode: config::InternalSpiMode,
    ) -> Result<Transceiver<'a, 'd, T, M, DataOnly, SpiCkoutMode, NeighborPins, Disabled>, Error>
    where
        SN: HasData,
    {
        let next_ch = <M::Next as TransceiverMarker>::CHANNEL.index();
        common.acquire_pin(next_ch, PinKind::Datin)?;

        self.select_channel_input(config::ChannelInput::Neighbor);
        self.select_data_mux_input(config::InputDataMux::ExternalSerial);
        self.select_serial_interface_type(mode.into());
        self.select_spi_clock(mode.into());
        Ok(Transceiver::new(common))
    }
}

// =============================================================================
// Single-use selectors
// =============================================================================

/// Per-transceiver pin selector. Cannot be constructed outside this module
/// (private field); handed out only inside the `configure_pins` closure,
/// one per transceiver, **by value**. Every method consumes `self`, so each
/// transceiver's pins can be declared exactly once (E0382 otherwise).
pub struct Sel<T: Instance, M: TransceiverMarker> {
    _m: PhantomData<(T, M)>,
}

impl<T, M> Sel<T, M>
where
    T: Instance,
    M: TransceiverMarker,
{
    pub(crate) fn new() -> Self {
        Self { _m: PhantomData }
    }
}

impl<'d, T, M> Sel<T, M>
where
    T: Instance,
    M: TransceiverMarker,
{
    /// Declare this transceiver with a DATIN pin (AF set here).
    pub fn datin(self, datin: Peri<'d, if_afio!(impl DatinPin<T, M, A>)>) -> DatinCfg<'d, T, M> {
        DatinCfg {
            datin: new_pin!(datin, AfType::input(Pull::None)).unwrap(),
            _m: PhantomData,
        }
    }

    /// Declare this transceiver with DATIN + CKIN.
    pub fn datin_ckin(
        self,
        datin: Peri<'d, if_afio!(impl DatinPin<T, M, A>)>,
        ckin: Peri<'d, if_afio!(impl CkinPin<T, M, A>)>,
    ) -> DckCfg<'d, T, M> {
        DckCfg {
            datin: new_pin!(datin, AfType::input(Pull::None)).unwrap(),
            ckin: new_pin!(ckin, AfType::input(Pull::None)).unwrap(),
            _m: PhantomData,
        }
    }

    /// Declare this transceiver as pinless (same as [`NoPinsCfg`]).
    pub fn none(self) -> NoPinsCfg {
        NoPinsCfg
    }
}

// =============================================================================
// Associate pin traits with transceivers
// =============================================================================

macro_rules! impl_ckin_bridge {
    ($marker:ty, $existing:ident) => {
        #[cfg(afio)]
        impl<T: Instance, A, P> CkinPin<T, $marker, A> for P
        where
            P: $existing<T, A>,
        {
            fn af_num(&self) -> u8 {
                $existing::af_num(self)
            }
        }

        #[cfg(not(afio))]
        impl<T: Instance, P> CkinPin<T, $marker> for P
        where
            P: $existing<T>,
        {
            fn af_num(&self) -> u8 {
                $existing::af_num(self)
            }
        }
    };
}

impl_ckin_bridge!(Tcv0, Ckin0Pin);
impl_ckin_bridge!(Tcv1, Ckin1Pin);
impl_ckin_bridge!(Tcv2, Ckin2Pin);
impl_ckin_bridge!(Tcv3, Ckin3Pin);
impl_ckin_bridge!(Tcv4, Ckin4Pin);
impl_ckin_bridge!(Tcv5, Ckin5Pin);
impl_ckin_bridge!(Tcv6, Ckin6Pin);
impl_ckin_bridge!(Tcv7, Ckin7Pin);

macro_rules! impl_datin_bridge {
    ($marker:ty, $existing:ident) => {
        #[cfg(afio)]
        impl<T: Instance, A, P> DatinPin<T, $marker, A> for P
        where
            P: $existing<T, A>,
        {
            fn af_num(&self) -> u8 {
                $existing::af_num(self)
            }
        }

        #[cfg(not(afio))]
        impl<T: Instance, P> DatinPin<T, $marker> for P
        where
            P: $existing<T>,
        {
            fn af_num(&self) -> u8 {
                $existing::af_num(self)
            }
        }
    };
}

impl_datin_bridge!(Tcv0, Datin0Pin);
impl_datin_bridge!(Tcv1, Datin1Pin);
impl_datin_bridge!(Tcv2, Datin2Pin);
impl_datin_bridge!(Tcv3, Datin3Pin);
impl_datin_bridge!(Tcv4, Datin4Pin);
impl_datin_bridge!(Tcv5, Datin5Pin);
impl_datin_bridge!(Tcv6, Datin6Pin);
impl_datin_bridge!(Tcv7, Datin7Pin);
