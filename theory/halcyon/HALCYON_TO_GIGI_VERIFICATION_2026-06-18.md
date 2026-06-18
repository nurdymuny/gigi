# Halcyon → GIGI verification verdict (2026-06-18)

**From:** Halcyon team (cross-team verification workflow `wyqh19me8`)
**To:** GIGI team
**Subject:** Part II HTTP × durable persistence verdict + two architectural asks
**Companion to:** `HALCYON_PART_I_GATES.md` section PART II, `HALCYON_TO_GIGI_REPLY_2026-06-17.md`, `HALCYON_PART_II_IMPLEMENTATION_LOG.md` section "Cross-team verification verdict (2026-06-18)"

Preserved here as the source-on-disk receipt for the architectural commitments the GIGI Part II implementation log cites. Per the closure discipline that put `HALCYON_TO_GIGI_REPLY_2026-06-17.md` on disk after the Q1–Q5 exchange: cross-team letters that drive commits in `main` belong in `theory/halcyon/` so future audits can verify the quote against the source.

---

## Verdict

**`HALCYON_AFFECTED_NONBLOCKING`.** Declare Part II done.

The one-line answer: the HTTP × durable persistence gap does not block Halcyon today — every Halcyon path (TDD scaffold, production orchestrator, Marcella read channel, Solves Vol 4 worked example) is either fully in-process or HTTP-read-only. But it forces one architectural commitment.

## The commitment

When the live-GIGI swap happens, it MUST be an **embedded PyO3 / CFFI binding**, not an HTTP client. Two independent reasons:

1. **Performance.** Per-sweep HTTP would dominate the 46-min production wall (O(10⁶) per-edge updates per run + JSON tax). Embedded is the only wire that preserves the current envelope.
2. **The gap.** HTTP-declare was in-memory only at the time of this letter (later closed in II.6b but still off the production hot path); embedded GQL is the path to `Engine::declare_*_durable`. Going embedded sidesteps the gap entirely.

The good news: Halcyon already wants embedded. The orchestrator does not import `gigi_client` yet, and the `Protocol` shape in `gigi_client/client.py` is RPC-agnostic — it is structural typing, not an HTTP stub. No rework needed when the binding swaps.

## Per-lens read

The verification ran four parallel impact lenses with a synthesizer:

| Lens | Verdict | Why |
| --- | --- | --- |
| TDD scaffold | `NOT_AFFECTED` | All 34 tests run intra-process against `MockGIGIClient`. Zero matches for `http` / `requests` / `subprocess` / `PERSIST` in `gigi_client/`. The contract is same-process bit-identity, which is independent of HTTP-vs-durable. |
| Production orchestrator | `NOT_AFFECTED` | `run_validation_report.py` cold-starts from `identity_links()` every run, writes one `final_state.npz` sidecar, exits. The gauge field never crosses a restart boundary. |
| Marcella + Solves narrative | `NONBLOCKING` | Marcella's GQL channel is read-only (`HOLONOMY` / `PLAQUETTE` / `MEASURE`) — HTTP-read is HTTP-safe by construction. Halcyon declares + persists embedded; Marcella reads over HTTP. Solves Vol 4 wants a frozen persisted corpus, not reader-side HTTP-declare. |
| 3 smaller follow-ups | `NONBLOCKING` | `test_G2_A` is the intended cross-engine pin for `INIT FROM` byte-equality — but it is *latent* (against the mock until the swap happens). Author-email drift and the `sudoku.rs:228` patent citation do not touch any Halcyon surface. |

## Halcyon-side actions

**Now (3 small, documentation-only):**

- Add a one-line note to `run_validation_report.py` near `build_graph`: *"this script does not persist or warm-start the gauge field; each run cold-starts from `identity_links()`"* — prevents a future contributor from adding `--resume` without realizing it would cross into the durable regime.
- Update `gigi_client/__init__.py` docstring: *"replacing `MockGIGIClient` with the live client is the only change Halcyon needs to make"* → add *"…assuming the live binding is embedded (PyO3 / CFFI). The HTTP surface is not on the production path; see `HALCYON_PART_I_GATES.md`."*
- In `HALCYON_PART_I_GATES.md` Part II receipts: name `test_G2_A_identity_field_round_trip` as the cross-engine contract pin for `INIT FROM` byte-equality, and note that its enforcement power is *contingent on the live-binding swap*. Discharges the post-Part-II completeness critic's concern about that contract being only latent.

**Soon (one check with the Marcella side):**

- Confirm Marcella's gauge-corpus query pattern is `SELECT` / `MEASURE` only — no `LATTICE` declare, `GAUGE_FIELD` declare, or `GIBBS_SAMPLE` over HTTP. The architecture memory framing her as read-only is 37 days old; if that changed, the verdict flips on her use case.

**Later:** byte-goldens to `gigi_client/golden/` (currently empty), and treat HTTP as debug-only.

## Two architectural asks back at GIGI

These tighten II.6's framing without re-opening the Part II sprint:

1. **Reframe II.6's "HTTP-declare is in-memory only" as a decision, not a deferred TODO.** The load-bearing declarer is Halcyon-embedded (or a one-shot CLI on embedded GQL). HTTP is the consumer surface. Saying that explicitly closes the gap as a design point rather than leaving it open as an apparent regression risk every future audit re-derives.
2. **Confirm `GIBBS_SAMPLE`-over-HTTP is also intentionally off the Part II surface.** It is a mutation; the durable-persistence gap would bite it the same way `DECLARE` does. Worth naming before the Marcella read pattern hardens.

Both asks can be absorbed into the Part II implementation log without opening new commits — though if the GIGI side prefers to wire HTTP × durable defensively (belt-and-suspenders), that is also a clean call.

---

## Bee's response (2026-06-18)

Belt-and-suspenders. Keep II.6b shipped (HTTP × `persist:true` wired end-to-end as defensive code), AND adopt the reframe in the impl log so the architectural commitment is canonical. Pre-commit `GIBBS_SAMPLE` to embedded-only in Part III.

Receipts:

- II.6b commit: `49bc52364`
- II.6c reframe commit: `d5d3853f5`
- This letter: `theory/halcyon/HALCYON_TO_GIGI_VERIFICATION_2026-06-18.md`
