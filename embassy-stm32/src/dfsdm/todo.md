# DFSDM — TODO

Open work across the embassy-stm32 driver and the stm32-data pipeline.
Closed work lives in `done.md`.

## Triage

1. **P1 — T-doc polish.** Remaining doc clauses (D2-D7, D9-D14), mostly one-line
   `# Note`s; D1 (liveness), D8 (ring layout) and FT8 (conversion time) are done.
2. **P2 — verify & examples.** `stm32mp157`, the FT5 break-caveat doc, five
   example gaps.
3. **P3 — reflect / optional.** NITS notes, FT19, housekeeping.
4. **Dormant / info (stm32-data).** SD6, SD9.

---

## P1 — T-doc polish (remaining doc clauses)

The deep docstring clauses are done: D1 (liveness contract), D8 (ring word
layout) and FT8 (conversion time), plus the `ResultRegular`/`ResultInjected`
`from_word` decoders. What remains is the lighter one-line clauses, written into
the relevant docstrings (rustdoc [`Type`] links, §-refs where the TRM is
load-bearing, `# Note`/`# Safety` rubrics; no em dashes; TRM content restated,
ASCII punctuation only):

- D2 — assign asymmetry: JCHGR instant + scan reset; RCH shadowed until next RSWSTART.
- D3 — `start_*`: requests ignored while RCIP/JCIP; injected preempts regular (restarts, flagged RPEND).
- D4 — DATINR: pre-start data lost; 16/32-bit accesses both legal (packing dependent).
- D5 — CKOUT sequencing: wait for CKOUT stopped before changing CKOUTSRC (glitch); stop timing 4 sysclk / 1 sysclk + 3 audio; 0-20 MHz.
- D6 — RCONT restart quirk: CR1 write with RCONT=1 mid-conversion restarts.
- D7 — disable semantics: DFEN=0 stops conversions, resets ISR+AWSR; whether RDATAR/JDATAR retain last value.
- D9 — break cross-link: DFSDM event→wire (BKSCD, BKAWH/BKAWL); TIM wire→BRK (FT5).
- D10 — ignore-overrun pattern on the unchecked read paths.
- D11 — ring: circular-only, one-ring-per-filter.
- D12 — CKAB held-set: raw 8-bit mask reads need the armed mask.
- D13 — extremes read-to-clear: `read_maxima`/`read_minima` reset EXMAX/EXMIN (+CH).
- D14 — AWFSEL coupling (per-channel fast filter only meaningful in fast mode) + AWFORD→AWFOSR.

---

## P2 — verify & examples

### Verify

- [ ] `stm32mp157` — no feature in this crate's Cargo.toml yet; revisit later.
- [ ] FT5 break-caveat doc: `set_break_dfsdm_enable`/`set_break2_dfsdm_enable`
  are unverified on F4 (both) and F7 (break2) — the TRMs (RM0402/RM0430/RM0410)
  don't document the AF1/AF2 DFSDM break bits. Document this in
  `timer/low_level.rs` (cross-ref from `complementary_pwm.rs`), and note the
  DFSDM source side (`ShortCircuitDetector::assign_break_signals` / AWD
  `BreakSignals`). TIM15/16/17 break remains unimplemented; the matrix doubles
  as its gate.

### Examples

- [ ] `dfsdm_parallel_dma_to_dma.rs`: exercise `read`/`blocking_read` with
  `Err(Overrun)` handling (API exists, usage not yet added).
- [ ] `dfsdm_it.rs`: `read(..)?`/Result handling.
- [ ] (stretch) AWD + SCD/CKAB guard example incl. `wait_for_sync()`.
- [ ] parallel-ADC example (`build_parallel_adc`).
- [ ] (stretch) `dfsdm_3phase_motor.rs`: PWM + DFSDM overcurrent break.

---

## P3 — reflect / optional

### NITS

- 11. `new_pin!(...).unwrap()` ×3 (transceiver.rs) — verify vs embassy conventions.
- 15. Reflect: `DFSDMEN` (peripheral enable) lives on `DfsdmCommon` — consider moving to the `Dfsdm` wrapper.
- 18. Packing-mode DATINR write restriction: `write_indat1` on a Standard-packed
  channel is a silent wrong write. Typemark the packing mode (or document).
- 19. (remaining half) dual-core `!Send` note (CR1 RMW is single-core only).
- 20. `set_continuous` straddles the config/runtime split (RCONT is runtime-writable); harmless, just the known exception.
- 21. `set_data_packing_mode` design musing: semantic `new_parallel_dma_dual()` pair constructor, or drop the comment.
- 22. `select_awd_filter_*` voluntary vs mandatory AWD fast-mode input-stage config.

### FT19 (optional) — where-cluster bundle

Condense the per-item `where` cluster
(`T: Instance + FilterInterrupt<M>, M: FilterMarker + InstanceEvents<T>`):

- **Mini-merge**: fold `InstanceEvents` into `FilterInterrupt` as
  `handle_instance_events()` (Flt0 real / Flt1..7 noop); deletes the sibling
  trait + `impl_noop_instance_events!`. Overturns TS5.
- **Marker-side bundle**: keep `InstanceEvents`, add
  `trait FilterFlow<T>: FilterMarker + InstanceEvents<T> {}` + blanket impl.

Cosmetic either way; default to leaving the cluster as-is unless it earns its churn.

### Housekeeping

- [ ] Chip-less embassy variants stay (no pruning yet): `DFSDM_2CH_1FLT_TRG3_ADC`,
  `DFSDM_2CH_1FLT_DLY_TRG5_ADC`, `DFSDM_4CH_2FLT_TRG3_ADC` (L451/452/462 are
  plain TRG3), `DFSDM_4CH_2FLT_DLY_TRG5_ADC_HWID` (MP13-only). Revisit later.

---

## Dormant / info (stm32-data)

Full detail lives in the stm32-data repo: `in_progress/DFSDMx/TODO.md`.

- [ ] **SD6 (dormant) — LPTIM3_ETR ← DFSDM2_BREAK0 (H7A/B).** Blocked on unmodeled LPTIM ETR input.
- [ ] **SD9 (info, no action) — MP13 chips absent.** Perimap regex correct but dormant.
