# DFSDM — TODO

Open work across the embassy-stm32 driver and the stm32-data pipeline.
Closed work lives in `done.md`.

## Postponed / revisit

- [ ] `stm32mp157` — no feature in this crate's Cargo.toml yet; revisit when MP1
  support lands in embassy.
- [ ] AWD + SCD/CKAB guard example — deferred (needs external-clock hardware for
  a meaningful `wait_for_sync`/CKAB demo; no CKIN available on this board).
- [ ] `dfsdm_3phase_motor.rs` — PWM + DFSDM overcurrent break; overkill for now,
  revisit as the end-to-end motor-control demo.
- [ ] Chip-less DFSDM variants in stm32-data stay unpruned:
  `DFSDM_2CH_1FLT_TRG3_ADC`, `DFSDM_2CH_1FLT_DLY_TRG5_ADC`,
  `DFSDM_4CH_2FLT_TRG3_ADC` (L451/452/462 are plain TRG3), and the MP13-only
  `DFSDM_4CH_2FLT_DLY_TRG5_ADC_HWID`. Cosmetic; revisit in stm32-data.

## Dormant / info (stm32-data)

Full detail lives in the stm32-data repo: `in_progress/DFSDMx/TODO.md`.

- [ ] **SD6 (dormant) — LPTIM3_ETR ← DFSDM2_BREAK0 (H7A/B).** Blocked on
  unmodeled LPTIM ETR input.
- [ ] **SD9 (info, no action) — MP13 chips absent.** Perimap regex correct but
  dormant.
