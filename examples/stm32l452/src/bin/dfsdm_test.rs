#![no_std]
#![no_main]
#![allow(dead_code)]
/// This example demonstrates how to use the QSPI peripheral in both indirect-mode and memory-mapped mode.
/// If you want to test this example, please pay attention to flash pins and check flash device datasheet
/// to make sure operations in this example are compatible with your device, especially registers I/O operations.
use embassy_stm32::dfsdm::config_types::{FilterOrder, FilterParameters};
use embassy_stm32::dfsdm::{FilterConfig, Flt0, Flt1, InjectedTrigger};
use embassy_stm32::peripherals::DFSDM1;
use embassy_stm32::qspi::{self, Instance};
use embassy_stm32::triggers::{TIM1_TRGO, TIM6_TRGO};
use embassy_stm32::{bind_interrupts, dfsdm, mode};
pub struct FlashMemory<I: Instance> {
    qspi: qspi::Qspi<'static, I, mode::Async>,
}
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    DFSDM1_FLT0 => dfsdm::InterruptHandler<DFSDM1, Flt0>;
    DFSDM1_FLT1 => dfsdm::InterruptHandler<DFSDM1, Flt1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // Start driver instantiation using DFSDM1 with a CKOUT pin on pin C2
    let dfsdm1 = dfsdm::Dfsdm::new(p.DFSDM1);

    let (common, split) = dfsdm1.configure_pins(|creator| {
        (
            creator.ch0.none(),
            creator.ch1.none(),
            creator.ch2.none(),
            creator.ch3.none(),
        )
    });

    let channel_mic = split
        .ch0
        .build_parallel_dma(&common, dfsdm::config_types::DataPackingModeReduced::Interleaved)
        .enable();

    let filter_params =
        FilterParameters::try_new(FilterOrder::Sinc3 { fosr: 100 }, 50).expect("This is inside the bounds");

    let flt_cfg = FilterConfig {
        // filter_cfg: FilterParameters::try_new(FilterOrder::Sinc3 { fosr: 5 }, 4).expect("This is inside the bounds"),
        filter_params,
        trigger: InjectedTrigger::from(TIM1_TRGO, dfsdm::config_types::TriggerEdge::Any),
        ..Default::default()
    };
    let flt_cfg2 = FilterConfig {
        // filter_cfg: FilterParameters::try_new(FilterOrder::Sinc3 { fosr: 5 }, 4).expect("This is inside the bounds"),
        filter_params,
        trigger: InjectedTrigger::from(TIM6_TRGO, dfsdm::config_types::TriggerEdge::Any),
        ..Default::default()
    };

    let mut _flt0 = split
        .flt0
        .build(&common, Irqs)
        .enable_no_dma(&channel_mic, [&channel_mic], &flt_cfg);
    let mut _flt0 = split
        .flt1
        .build(&common, Irqs)
        .enable_no_dma(&channel_mic, [&channel_mic], &flt_cfg2);

    loop {
        Timer::after_millis(1000).await;
    }
}
