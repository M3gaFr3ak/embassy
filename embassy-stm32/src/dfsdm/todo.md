# DFSDM — TODO

Open work across the embassy-stm32 driver and the stm32-data pipeline.
Closed work lives in `done.md`.

## Triage

1. **P1 — substantive.** FT22 (build.rs capability derivation), T-doc
   (D-clause docstring semantics). These are the two items with real scope.
2. **P2 — verification & examples.** Clippy/mp157 on the matrix, the five
   example gaps, the FT5 (TIM15/16/17) gate.
3. **P3 — optional / reflect.** NITS reflect & hardening notes, FT19/FT9
   (cosmetic helpers), housekeeping.
4. **Dormant / info (stm32-data).** SD6, SD9, timer-break research.

---

## P1 — substantive

### FT22 — derive instance capabilities from the block name

String-match `regs.block` in build.rs instead of the `mark_dfsdm_instances!`
table. The 13 DFSDM block names form a closed grammar
`DFSDM_{2,4,8}CH_{1,2,4,6,8}FLT[_DLY]_TRG{3,5}[_ADC][_HWID]`, and build.rs
already holds `regs.block`. Plan:

- build.rs string-matches `regs.block` → `Transceivers`/`Filters`/`HasDelay`/
  `HasHwid`/`AdcInput`, then emits `impl SealedInstance` + `impl Instance` +
  capability flags directly (drops `mark_dfsdm_instances!` +
  `impl_dfsdm_instance!`).
- Drop `Instance::Repr` (dead: declared + assigned, never read; the driver
  only touches `DfsdmSuperset`).
- **Interrupts stay in `foreach_interrupt!`**: make the IRQ binding
  count-agnostic by unconditionally binding `Flt0..Flt7` (each
  `foreach_interrupt!` arm matches only the `FLTx` rows the chip actually has),
  removing `dfsdm_flt_irqs!` and the filter-count dependency from the
  interrupt side.
- **Neighbor ring stays** (`impl_next_channel!` + the splits.rs S-pairing):
  chip-independent modulo-N successor, already once-per-arity, no string to
  match — not a capability table, out of scope here.

### T-doc — full docstring pass (D-clause semantics)

Absorbs D1–D13 + D14's doc clauses, FT8, FT15's doc clauses, NITS #3/#8.
Every public item gets a real docstring; the missing_docs warnings are the
inventory, so nothing is skipped. Formalize into rustdoc ([`Type`] links,
§-refs where the TRM is load-bearing, `# Note`/`# Safety` rubrics; no em
dashes). TRM content is restated, not copied verbatim: ASCII punctuation only
(`->` not `→`, `x` not `×`, `<=` not `≤`); `§` is kept for section references
only. Batched per module (types.rs, splits.rs, detector objects, read
paths, dma.rs) with a missing_docs-per-module zero gate. The former D-register
notes become the docstrings they annotate:

- D1 — liveness contract on the filter read paths ("read() hangs silently iff
  the source is starved"; layered detection: borrow-connected transceivers,
  CKAB (FT3), CNVTIMR+timeout, overrun (FT1)).
- D2 — assign-as-overwrite asymmetry: JCHGR instant + scan reset; RCH shadow
  applied at next RSWSTART.
- D3 — `start_*` semantics: requests ignored while RCIP/JCIP; regular
  interrupted by injected restarts later, flagged by RPEND.
- D4 — DATINR: pre-start data lost; 16- and 32-bit accesses both legal
  (packing-mode dependent).
- D5 — CKOUT sequencing: wait for CKOUT stopped before changing CKOUTSRC
  (glitch); stop timing 4 sysclk / 1 sysclk + 3 audio clk; 0-20 MHz range.
- D6 — RCONT restart quirk: CR1 write with RCONT=1 mid-conversion restarts
  from the next conversion cycle.
- D7 — disable semantics: DFEN=0 stops conversions and resets ISR + AWSR;
  answer whether RDATAR/JDATAR retain their last value (doc).
- D8 — ring word layout: one u32 = `RDATA[23:8] | RPEND | RDATACH` (JDATA
  analog); channel byte load-bearing for scan demux; 32-bit only.
- D9 — break cross-link: DFSDM = event→wire (BKSCD, BKAWH/BKAWL,
  `BreakSignals`); TIM = wire→BRK enable (FT5).
- D10 — ignore-overrun pattern on `get_*_unchecked` (both halves): unchecked +
  EOC/JEOC poll = "always-fresh, overruns don't matter"; FT1's `Err(Overrun)`
  for callers who care.
- D11 — ring docs: circular-only, one-ring-per-filter (F6; "use two filters
  for both").
- D12 — CKAB held-set note (E2): raw 8-bit mask reads need the armed mask.
- D13 — extremes read-to-clear: `read_maxima`/`read_minima` reset EXMAX/EXMIN
  (+ CH fields) on read.
- D14 (doc clauses) — AWFSEL coupling ("per-channel filter only meaningful in
  fastmode") + the "AWFORD"→"AWFOSR" docstring fix, under FT21's new names.
- FT8 — CNVTIMR "measures filter activity, not consumer progress" + the
  starvation recipe (timeout + two Δt reads: frozen = starved, advancing =
  alive-but-slow) as the `conversion_time()` docstring.
- NITS #3 — populate `Config` docs (mod.rs:44-48) or remove the struct.

Status (2026-09): the module-restructure doc pass landed (every public item
carries a docstring; `missing_docs` gate = 0 warnings). Still open is the deep
D-clause semantics — D1 liveness, D8 ring layout, FT8's CNVTIMR starvation
recipe, and the §-ref `# Note`/`# Safety` rubrics. That is the one remaining
substantive doc item.

---

## P2 — verification & examples

### Verify (remaining)

- [ ] `cargo clippy` on the DFSDM chip matrix (`cargo check` + `cargo fmt` are
  already green).
- [ ] `stm32mp157` — no feature in this crate's Cargo.toml yet; revisit later.
- [ ] FT5 gate: the matrix doubles as the gate for the optional TIM15/16/17
  break impl.

### Examples

- [ ] `dfsdm_parallel_dma_to_dma.rs`: exercise `read` (async) / `blocking_read`
  with `Err(Overrun)` handling — the API exists (FT2), usage in the example
  still to be added.
- [ ] `dfsdm_it.rs` → `read(..)?`/Result handling.
- [ ] New (stretch): AWD + SCD/CKAB guard example incl. `wait_for_sync()` arm
  sequence.
- [ ] New: parallel-ADC example (`build_parallel_adc`) — internal-ADC input,
  complementing the existing `dfsdm_parallel_dma_to_dma.rs` CPU/DMA path.
- [ ] `dfsdm_3phase_motor.rs` (stretch): 3-phase PWM + DFSDM — injected
  conversions TRGO-triggered → injected ring (circular), controller reads in
  PWM-center ISR; regular continuous + manual latest reads (ignore overrun,
  D10); AWD fast-mode high threshold → break0 → TIMx BRK (hardware overcurrent
  break) on one filter/channel, AWD IRQ-only on others (graceful shutdown);
  CKAB via FT3/FT12.

---

## P3 — optional / reflect

### NITS

5. AFS critical-section question (mod.rs:103, `// TODO MAYBE USE CRITICAL
   SECTION FOR AFS?!`) — fold into F1 (one critical_section strategy for all
   RMW: CR2 + AF assignment). The configure_pins AF assignment is one-shot
   (not ISR RMW), so it likely needs no guard — decide and drop the comment.
11. `new_pin!(...).unwrap()` ×3 (transceiver.rs:651/663/664) — verify vs
    embassy conventions.
15. Reflect: `DFSDMEN` (peripheral enable) currently lives on `DfsdmCommon` —
    consider whether the global enable belongs on the `Dfsdm` wrapper instead.
17. `FilterConfig::default()` (filter.rs:34) calls
    `FilterParameters::new(Disabled, 1)` — its `.expect` is provably
    unreachable (`Disabled` → fosr=1, gain=1, total gain=1 ≤ MAX_GAIN; iosr=1
    in 1..=256), so the default can never panic. Add a comment documenting that
    invariant (or an infallible const default path).
18. Packing-mode DATINR write restriction: `write_sample_standard` (INDAT0) and
    `write_indat1` (INDAT0+INDAT1) are both exposed on every `ParallelDmaMode`
    transceiver regardless of DATPACK — `write_indat1` on a Standard-packed
    channel is a silent wrong write. Typemark the packing mode (or document
    which writer matches which `DataPackingModeReduced`).
19. Deferred hardening: `#[diagnostic::on_unimplemented]` on the DMA-channel
    binding ("DMAx_CHy cannot service DFSDM filter M {regular|injected}"); and
    a dual-core `!Send` note (CR1 RMW is safe single-core only). Low priority.
20. `set_continuous` straddles the config/runtime split: it's the one
    `FilterDisabled` static (filter.rs:225) that is also runtime-reachable,
    since RCONT is runtime-writable — `FilterRegular::set_continuous(&mut
    self)` (filter.rs:526) delegates back into the Disabled-scoped static,
    while the other six config statics are DFEN=0-gated only. Harmless (thin
    delegate); just the known exception.
21. `set_data_packing_mode` design musing (transceiver.rs:513) — consider a
    semantic dual-pair constructor — `new_parallel_dma_dual()` on the even
    channel meaning "this channel and its paired successor are configured as a
    dual-input pair" — folding the comment's intent into the API or deleting
    the comment. Decide during the FT7/FT18 API pass.
22. `select_awd_filter_order`/`select_awd_filter_osr` (transceiver.rs:182/191,
    renamed per FT21) are pub consuming builders (voluntary — AWFORD/AWFOSR
    stay at reset if untouched). Think about whether the AWD fast-mode
    input-stage config should be **mandatory** at build time instead (required
    constructor param or configure step), so it can't be forgotten when AWFSEL
    fastmode is intended. Voluntary by decision for now.

### FT19 (low priority) — bundle ergonomics, re-approach

Decide later between two idioms for condensing the per-item `where` cluster
(`T: Instance + FilterInterrupt<M>, M: FilterMarker + InstanceEvents<T>`):

- **Mini-merge**: fold `InstanceEvents` into `FilterInterrupt` as an assoc fn
  `handle_instance_events()` (Flt0 real / Flt1..7 noop, emitted in
  `impl_dfsdm_filter_irq!`); deletes the sibling trait and the
  `impl_noop_instance_events!` macro; header becomes `T: FilterInstance<M>`
  -friendly. Overturns TS5's "don't merge" note.
- **Marker-side bundle (optional alternative)**: keep `InstanceEvents`, but
  `trait FilterFlow<T>: FilterMarker + InstanceEvents<T> {}` + blanket impl, so
  `M: FilterFlow<T>` elaborates both via supertraits (rust#20671 behavior);
  headers read `T: Instance + FilterInterrupt<M>, M: FilterFlow<T>`.
- Either is cosmetic; default to leaving the cluster as-is if neither earns its
  churn. Verify empirically (playground + chip matrix) before committing.

### FT9 (optional) — `CkoutDivider::for_manchester(rate)` helper

From the RM0455 Manchester formula:
`(CKOUTDIV+1)·T_INCKOUT < T_manchester < 2·CKOUTDIV·T_INCKOUT`.

### Housekeeping

- [ ] Chip-less embassy variants stay for now — decision: **no pruning yet**.
  After the stm32-data fixes, these still have no owning chip:
  `DFSDM_2CH_1FLT_TRG3_ADC`, `DFSDM_2CH_1FLT_DLY_TRG5_ADC`,
  `DFSDM_4CH_2FLT_TRG3_ADC` (L451/452/462 are plain TRG3 per rm0394);
  `DFSDM_4CH_2FLT_DLY_TRG5_ADC_HWID` is MP13-only (no MP13 chips in the chip
  db). Revisit later.

---

## Dormant / info (stm32-data)

Full detail lives in the stm32-data repo: `in_progress/DFSDMx/TODO.md`.

- [ ] **SD6 (dormant) — LPTIM3_ETR ← DFSDM2_BREAK0 (H7A/B).** Blocked on
  unmodeled LPTIM ETR input signal; not blocking anything else.
- [ ] **SD9 (info, no action) — MP13 chips absent.** Perimap regex correct but
  dormant.

### Research note — timer break bits (F4/F7)

Following have no bken enable for dfsdm bits in timers. Do research:

- rm0394 — STM32L41x/42x/43x/44x/45x/46x
- rm0402 — STM32F412
- rm0410 — STM32F76x/77x
- rm0430 — STM32F413/423
