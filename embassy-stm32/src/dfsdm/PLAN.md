# DFSDM — Implementation plan

Ordered priority list across both repos (embassy driver + stm32-data pipeline).
Companion to `TODO-v2.md` (HAL side) and `stm32-data/in_progress/DFSDMx/TODO.md`
(data side).

---

## Phase 1 — Quick cleanups (embassy)

Small, independent, safe-to-delete/rename items.

[x] 1. NITS #16 — delete `DataSource`/`ExternalSource`/`InternalSource`, delete
   `NotFlt0`, rename `_datasource_marker` → `_channel_mode_marker`.
[x] 2. F2 — delete `get_datinr_as_ref` (unsound `&self -> &mut u32`).
[x] 3. NITS #10 — delete `FilterTrait`.
[X] 4. FT16 — replace `UInt<BITS,T>` with `DataRightShift` + `PulsesToSkip`.
[X] 5. NITS #1/#2/#13 — doc typos + receiver-consistency sweep.

## Phase 2 — Type system + interrupt restructure (embassy)

[X] 6. FT17 — closed. TS1 done (PinSource axis); TS7 SAFETY review done (sound);
   TS2/TS3/TS4 declined; TS5 kept as note; TS6 rejected (two-lifetime `'a`/`'d`
   is load-bearing — see TODO-v2 TS6).
[X] 7. FT18 (`IR1`–`IR5`) — interrupt binding at construction, single idempotent
   NVIC enable.

## Phase 3 — Detector redesign (embassy)

[X] 9. FT12 — AWD-style SCD/CKAB detector objects (declared priority 1).
   Residue routed: public `flags()`/`clear_flags()` renames → item 12
   (FT4/FT11); `common.detectors()`/DetectorsBuilder-drop closed as
   superseded (binding-at-build already satisfied).
[X] 10. F1 — CR2 RMW race fix, done alongside FT12 (same code paths).

## Phase 4 — Data path (embassy)

Sequential — each depends on the previous.

[X] 13. F7 (2FLT slice) — DONE. `capability::Flt2`/`Flt6` + `FilterCount`
    impls; IRQ sets derived from the new `dfsdm_flt_irqs!` capability-token
    template (associations.rs, replaces all per-variant manual lists);
    `define_dfsdm_ready!` bundles; the split structs generated from the new
    splits.rs shape tables (see TODO FT20 — the work outgrew the slice);
    verified against the extracted RM0402/RM0430 DFSDM chapters.
    stm32f412zg / stm32f413zh compile (was: E0425 at the old associations
    list; perimap maps F412/F413 to DFSDM_4CH_2FLT_TRG3). 6FLT residue:
    chip availability only (MP13, rides Phase 6).
[X] 20. FT20 — splits.rs shape-table types (split + select bundles),
    `define_dfsdm_ready!` where-clause bundles, selector generation with the
    `Shape` impls moved out of types.rs. Done with item 13; details in TODO.
11. [X] F3 + [X] F4 + [X] FT2 + [X] FT1 — DONE, see TODO as-built (gating is
    free from the F4 restructure; `RingBufferedFilter<'e, T, M, DM>` with
    half-methods `ring_buffered`; `FilterDma` trait retained by decision;
    ring read API is thin delegates + error canonicalization +
    construction-time alignment + `start_conversion()` forwarder; overrun
    propagation complete with filter-side and DMA-side detection).
12. [X]FT3 + [X]FT4 + [X]FT11 — `wait_for_sync`, `clear_flags`, ISR accessors.

## Phase 5 — Peripheral features (embassy)

Independent, no ordering constraints.

14. [X]FT5 (TIM break), [X]FT7 (delay block), [X]FT6 (HWID), [X]FT13/[X]FT14 (gain ceilings),
    [X]FT15 (API polish), [X]FT21 (AWD renames). (FT8's doc part absorbed
    into T-doc.)

## Phase 6 — stm32-data

15. [X] SD10 first (PRIORITY 1) — 3-bit-JEXTSEL trigger suffix renumbering. Pure
    data rename, no embassy change.
    DONE via the trigger rework: no data renumber — build.rs emits `TriggerSource`
    impls directly (identity on TRG5, remap from `src/dfsdm/trigger_map.rs` on TRG3).
16. [X] SD1–SD5 (chip-unlock batch) — header `_NS` (SD1), F7 regex (SD2), L4
    regexes (SD3), H7B0 (SD4), F413 JTRG names (SD5). DONE.
17. [X] Regenerate data + metapac; verify newly-enabled chips (F777-779,
    L451/452/462, L471-486, L552/562, H7B0). DONE — 394 DFSDM chips, 0
    unmapped, all 13 blocks correct; metapac dedup 0 misses.

## Phase 7 — Deferred / minor / info

18. SD6 (dormant) LPTIM3_ETR, SD9 (info) MP13 absent. SD7 (minor) MP1 ADFSDM
    bits — already present in `rcc_mp1.yaml` (no-op). SD8 (minor) H7A/B DFSDM2
    kernel mux — DONE (added `DFSDM2SEL` enum, kernel clock now muxed).
19. F7 remainder (6FLT: fully defined — MP13-only, no chip in the db),
    EXAMPLES, T-doc (absorbs D1–D14 docs), VERIFY matrix. FT10 (2CH_1FLT) and
    FT21 (AWD renames) since DONE — see TODO-v2 DONE.
20. [ ] FT22 — derive instance capabilities by string-matching `regs.block` in
    build.rs (drop `mark_dfsdm_instances!` + `impl_dfsdm_instance!` + dead
    `Instance::Repr`). Interrupts stay in `foreach_interrupt!` (made
    count-agnostic); the neighbor ring stays (chip-independent, once-per-arity).
    Detail in TODO-v2.md FT22.

---

## Docstring pass — single consolidated item (do last)

21. T-doc — full docstring rewrite of the dfsdm module, absorbing D1–D14's
    remaining doc clauses, the doc-note halves of FT8/FT15, and NITS #3/#8
    (runs once, AFTER the Phase 4 runtime arc: FT2/FT1 and the
    FT3/FT4/FT11 batch land, and after the FT15/FT21 renames settle the
    API shapes - docstrings get written once).
    Workflow: per-keyword-card prompting (describe item + hardware
    contract + safety caveats; missing_docs warnings are the inventory,
    so nothing is skipped) -> human prose from the author -> formalized
    into rustdoc ([`Type`] links, §-refs where the TRM is load-bearing,
    `# Note`/`# Safety` rubrics; no em dashes). Batches by module:
    types.rs, splits.rs, detector objects, read paths, dma.rs; the former
    D-register notes become the docstrings of their subjects
    (D1 liveness, D2 asymmetry, D3 start_*, D4 DATINR, D5 CKOUT, D6 RCONT,
    D7 disable, D8 ring layout, D9 break link, D10 ignore-overrun,
    D11 circular-only, D12 CKAB held-set, D13 extremes read-to-clear,
    FT8's CNVTIMR starvation recipe = the `conversion_time()` docstring).
    Per-module gate: missing_docs count for that module reaches 0.
    D14's rename half is CODE and stays open separately (FT21); only its
    doc clauses ride T-doc.

### Optional
[ ]FT9

---

## Notes

- Phase 6 is the only cross-repo work; batch it separately from the
  embassy-side items.
- Phases 1–2 are good standalone commits; Phase 4 should land as one coherent
  change set.
- SD6–SD9 are non-blocking (dormant/minor/info) and excluded from the
  regen-gate.
- NITS #20 — `set_continuous` is the one config static also reachable at
  runtime (RCONT is runtime-writable); harmless, just the known exception.