#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::dfsdm::config::{DataRightShift, FilterOrder, FilterParameters};
use embassy_stm32::dfsdm::{FilterConfig, Flt0, Flt1};
use embassy_stm32::dma::{self, Channel, Transfer, TransferOptions};
use embassy_stm32::peripherals::{self, DFSDM1};
use embassy_stm32::{SharedData, bind_interrupts, dfsdm};
use panic_probe as _;

#[unsafe(link_section = ".ram_d3.shared_data")]
static SHARED_DATA: MaybeUninit<SharedData> = MaybeUninit::uninit();

bind_interrupts! (struct Irqs{
    DFSDM1_FLT0 => dfsdm::InterruptHandler<DFSDM1, Flt0>;
    DFSDM1_FLT1 => dfsdm::InterruptHandler<DFSDM1, Flt1>;
    MDMA => dma::InterruptHandler<peripherals::MDMA_CH0>;
    DMA1_STREAM0 => dma::InterruptHandler<peripherals::DMA1_CH0>;
    DMA1_STREAM1 => dma::InterruptHandler<peripherals::DMA1_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = Some(HSIPrescaler::Div1);
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::Hsi,
            prediv: PllPreDiv::Div4,
            mul: PllMul::Mul50,
            divp: Some(PllDiv::Div2),
            divq: Some(PllDiv::Div8), // 100mhz
            divr: None,
        });
        config.rcc.sys = Sysclk::Pll1P; // 400 Mhz
        config.rcc.ahb_pre = AHBPrescaler::Div2; // 200 Mhz
        config.rcc.apb1_pre = APBPrescaler::Div2; // 100 Mhz
        config.rcc.apb2_pre = APBPrescaler::Div2; // 100 Mhz
        config.rcc.apb3_pre = APBPrescaler::Div2; // 100 Mhz
        config.rcc.apb4_pre = APBPrescaler::Div2; // 100 Mhz
        config.rcc.voltage_scale = VoltageScale::Scale1;
        config.rcc.supply_config = SupplyConfig::DirectSMPS;
    }

    //==================================================
    // Goal: feed a fixed dual-packed sequence into DFSDM via MDMA mem2mem and
    // read both channels of the dual pair.
    //==================================================
    let p = embassy_stm32::init_primary(config, &SHARED_DATA);
    info!("Hello World!");

    // Start driver instantiation using DFSDM1 with a CKOUT pin on pin C2
    let dfsdm1 = dfsdm::Dfsdm::new(p.DFSDM1);

    let (common, split) = dfsdm1.configure_pins(|creator| {
        (
            creator.ch0.none(),
            creator.ch1.none(),
            creator.ch2.none(),
            creator.ch3.none(),
            creator.ch4.none(),
            creator.ch5.none(),
            creator.ch6.none(),
            creator.ch7.none(),
        )
    });

    // Dual pair: even = ch0 (owns DATINR), odd = ch1 (fed by the auto-copy).
    let pair = split
        .ch0
        .build_parallel_dual(&common, split.ch1)
        .set_data_right_shift([DataRightShift::new(0); 2])
        .enable();

    let filter_params = FilterParameters::try_new(FilterOrder::Disabled, 32).expect("This is inside the bounds");

    let flt_cfg0 = FilterConfig::<DFSDM1, Flt0> {
        filter_params,
        enable_continuous_regular: true,
        enable_fast_regular: true,
        ..Default::default()
    };
    let flt_cfg1 = FilterConfig::<DFSDM1, Flt1> {
        filter_params,
        enable_continuous_regular: true,
        enable_fast_regular: true,
        ..Default::default()
    };

    // One filter per channel: flt0 reads the even channel, flt1 the odd channel.
    let mut flt0 = split
        .flt0
        .build(&common, Irqs)
        .enable_reg_dma(&pair.even, [&pair.even], &flt_cfg0);
    let mut flt1 = split
        .flt1
        .build(&common, Irqs)
        .enable_reg_dma(&pair.odd, [&pair.odd], &flt_cfg1);

    let mut buffer_even = [0u32; 32];
    let mut buffer_odd = [0u32; 32];

    let mut ring_even = flt0.regular.ring_buffered(p.DMA1_CH0, Irqs, &mut buffer_even);
    let mut ring_odd = flt1.regular.ring_buffered(p.DMA1_CH1, Irqs, &mut buffer_odd);

    ring_even.start();
    ring_odd.start();
    ring_even.start_conversion();
    ring_odd.start_conversion();

    // Each u32 word packs two 16-bit samples: INDAT0 (even) in the low half,
    // INDAT1 (odd) in the high half.
    let source: [u32; 32 * 4] = core::array::from_fn(|i| (i as u32).wrapping_mul(0x01010101) ^ 0xDEADBEEF);

    let mut dma_ch = Channel::new(p.MDMA_CH0, Irqs);
    let tfer_opts = TransferOptions::default();

    println!("DMA starting...");

    // No request number needed as MEM2MEM transfers on MDMA are software-controlled. Defaulting to 0.
    let tfer: Transfer<'_> =
        unsafe { dma_ch.write_mem2mem::<u32, u32>(0, &source, pair.get_datinr_as_ptr(), tfer_opts) };
    tfer.await; // theoretically unnecessary

    println!("DMA finished.");

    let mut result_even = [0u32; 32];
    let mut result_odd = [0u32; 32];

    loop {
        let amount_even = ring_even.read_latest(&mut result_even).unwrap();
        let amount_odd = ring_odd.read_latest(&mut result_odd).unwrap();
        if amount_even > 0 || amount_odd > 0 {
            println!("even: {} samples, odd: {} samples", amount_even, amount_odd);
        }
    }
}
