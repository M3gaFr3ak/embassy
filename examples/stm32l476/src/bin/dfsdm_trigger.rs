#![no_std]
#![no_main]
#![allow(dead_code)]

//! Bare-bones DFSDM injected-trigger test on a 4-filter part (STM32L476,
//! `DFSDM_8CH_4FLT_TRG3`).
//!
//! On 3-bit-JEXTSEL parts the `JTRGn` channel number is not the register
//! value — each filter compacts a *different* subset of the trigger channels
//! into `0..7` (see `embassy-stm32/src/dfsdm/trigger_map.rs`). `Flt0`/`Flt1`
//! share one set; `Flt2`/`Flt3` each diverge. This example assigns a distinct
//! trigger per filter and logs the resolved `jextsel` value. Invalid
//! (filter, trigger) pairs are rejected at compile time through the
//! `TriggerSource<T, M>` bound.

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::dfsdm::config_types::{FilterOrder, FilterParameters, TriggerEdge};
use embassy_stm32::dfsdm::{FilterConfig, Flt0, Flt1, Flt2, Flt3, InjectedTrigger, TriggerSource};
use embassy_stm32::peripherals::DFSDM1;
use embassy_stm32::triggers::{TIM1_TRGO, TIM3_TRGO, TIM6_TRGO, TIM7_TRGO};
use embassy_stm32::{bind_interrupts, dfsdm};
use embassy_time::Timer;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    DFSDM1_FLT0 => dfsdm::InterruptHandler<DFSDM1, Flt0>;
    DFSDM1_FLT1 => dfsdm::InterruptHandler<DFSDM1, Flt1>;
    DFSDM1_FLT2 => dfsdm::InterruptHandler<DFSDM1, Flt2>;
    DFSDM1_FLT3 => dfsdm::InterruptHandler<DFSDM1, Flt3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

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

    // One parallel-DMA channel shared by all four filters (this test only
    // exercises the per-filter JEXTSEL mapping, not the datapath).
    let channel = split
        .ch0
        .build_parallel_dma(&common, dfsdm::config_types::DataPackingModeReduced::Interleaved)
        .enable();

    let filter_params = FilterParameters::try_new(FilterOrder::Sinc3 { fosr: 100 }, 50).unwrap();

    // Per-filter triggers. `TIM6_TRGO` (JTRG7) and `TIM1_TRGO` (JTRG0) are the
    // common Flt0/Flt1 set; `TIM7_TRGO` (JTRG8) exists only on Flt2/Flt3;
    // `TIM3_TRGO` (JTRG4) only on Flt3. A wrong pair is a compile error.
    let flt0_cfg = FilterConfig {
        filter_params,
        trigger: InjectedTrigger::from(TIM6_TRGO, TriggerEdge::Any), // -> jextsel 5
        ..Default::default()
    };
    let flt1_cfg = FilterConfig {
        filter_params,
        trigger: InjectedTrigger::from(TIM1_TRGO, TriggerEdge::Any), // -> jextsel 0
        ..Default::default()
    };
    let flt2_cfg = FilterConfig {
        filter_params,
        trigger: InjectedTrigger::from(TIM7_TRGO, TriggerEdge::Any), // -> jextsel 5
        ..Default::default()
    };
    let flt3_cfg = FilterConfig {
        filter_params,
        trigger: InjectedTrigger::from(TIM3_TRGO, TriggerEdge::Any), // -> jextsel 3
        ..Default::default()
    };

    let _flt0 = split
        .flt0
        .build(&common, Irqs)
        .enable_no_dma(&channel, [&channel], &flt0_cfg);
    let _flt1 = split
        .flt1
        .build(&common, Irqs)
        .enable_no_dma(&channel, [&channel], &flt1_cfg);
    let _flt2 = split
        .flt2
        .build(&common, Irqs)
        .enable_no_dma(&channel, [&channel], &flt2_cfg);
    let _flt3 = split
        .flt3
        .build(&common, Irqs)
        .enable_no_dma(&channel, [&channel], &flt3_cfg);

    log_trigger::<DFSDM1, Flt0, _>("flt0", &TIM6_TRGO);
    log_trigger::<DFSDM1, Flt1, _>("flt1", &TIM1_TRGO);
    log_trigger::<DFSDM1, Flt2, _>("flt2", &TIM7_TRGO);
    log_trigger::<DFSDM1, Flt3, _>("flt3", &TIM3_TRGO);

    loop {
        Timer::after_secs(1).await;
    }
}

fn log_trigger<T, M, TR>(label: &str, trg: &TR)
where
    T: dfsdm::Instance,
    M: dfsdm::FilterMarker,
    TR: TriggerSource<T, M>,
{
    info!("{}: jextsel = {}", label, trg.jextsel());
}
