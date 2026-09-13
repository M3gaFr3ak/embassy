use super::types::*;

// =============================================================================
// Associate pin traits with channels
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

// =============================================================================
// Associate interrupts
// =============================================================================

// Implement single IRQ for a single filter
macro_rules! impl_dfsdm_filter_irq {
    ($inst:ident, $filter:ty, $irq:ident) => {
        impl FilterInterrupt<$filter> for crate::peripherals::$inst {
            type Interrupt = crate::interrupt::typelevel::$irq;

            fn state() -> &'static State {
                static STATE: State = State::new();
                &STATE
            }
        }
    };
}

// Bind every filter interrupt the chip actually exposes. The `foreach_interrupt!`
// catch-all skips FLTx rows a given chip doesn't have, so the filter count is
// irrelevant here — no per-variant dispatch.
foreach_interrupt! {
    ($inst:ident, dfsdm, $variant:ident, FLT0, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt0, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT1, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt1, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT2, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt2, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT3, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt3, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT4, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt4, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT5, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt5, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT6, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt6, $irq);
    };
    ($inst:ident, dfsdm, $variant:ident, FLT7, $irq:ident) => {
        impl_dfsdm_filter_irq!($inst, Flt7, $irq);
    };
}
