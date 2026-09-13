//! 3-bit JEXTSEL remap table for DFSDM injected triggers.
//!
//! On 3-bit-JEXTSEL parts the `DFSDM1_JTRGn` channel number is *not* the
//! register value; the valid channels are compressed into 0..7 per filter.
//! Each row is `(filter index, jtrg channel number, jextsel)`.
//!
//! This file is read by `build.rs` (via `#[path]`) to emit the per-source
//! `TriggerSource` impls, so it must stay free of any target-specific types.

pub const DFSDM_TRG3_JEXTSEL: &[(u8, u8, u8)] = &[
    (0, 0, 0),
    (0, 1, 1),
    (0, 2, 2),
    (0, 3, 3),
    (0, 5, 4),
    (0, 7, 5),
    (0, 9, 6),
    (0, 10, 7),
    (1, 0, 0),
    (1, 1, 1),
    (1, 2, 2),
    (1, 3, 3),
    (1, 5, 4),
    (1, 7, 5),
    (1, 9, 6),
    (1, 10, 7),
    (2, 0, 0),
    (2, 1, 1),
    (2, 2, 2),
    (2, 3, 3),
    (2, 5, 4),
    (2, 8, 5),
    (2, 9, 6),
    (2, 10, 7),
    (3, 0, 0),
    (3, 1, 1),
    (3, 2, 2),
    (3, 4, 3),
    (3, 6, 4),
    (3, 8, 5),
    (3, 9, 6),
    (3, 10, 7),
];
