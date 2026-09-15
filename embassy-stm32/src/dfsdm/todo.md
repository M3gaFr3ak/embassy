# DFSDM — TODO

Open work across the embassy-stm32 driver and the stm32-data pipeline.
Closed work lives in `done.md`.

## Triage

1. **P2 — examples (stretch/optional).** `stm32mp157` (defer) and three
   optional example ideas.
2. **P3 — housekeeping (stm32-data).** Chip-less DFSDM variants stay unpruned.
3. **Dormant / info (stm32-data).** SD6, SD9.

---

## P2 — verify & examples

### Verify

- [ ] `stm32mp157` — no feature in this crate's Cargo.toml yet; revisit later.

### Examples (stretch / optional)

- [ ] AWD + SCD/CKAB guard example incl. `wait_for_sync()`.
- [ ] parallel-ADC example (`build_parallel_adc`).
- [ ] `dfsdm_3phase_motor.rs`: PWM + DFSDM overcurrent break.

---

## P3 — housekeeping (stm32-data)

- [ ] Chip-less embassy variants stay (no pruning yet): `DFSDM_2CH_1FLT_TRG3_ADC`,
  `DFSDM_2CH_1FLT_DLY_TRG5_ADC`, `DFSDM_4CH_2FLT_TRG3_ADC` (L451/452/462 are
  plain TRG3), `DFSDM_4CH_2FLT_DLY_TRG5_ADC_HWID` (MP13-only). Revisit later.

---

## Dormant / info (stm32-data)

Full detail lives in the stm32-data repo: `in_progress/DFSDMx/TODO.md`.

- [ ] **SD6 (dormant) — LPTIM3_ETR ← DFSDM2_BREAK0 (H7A/B).** Blocked on
  unmodeled LPTIM ETR input.
- [ ] **SD9 (info, no action) — MP13 chips absent.** Perimap regex correct but
  dormant.
