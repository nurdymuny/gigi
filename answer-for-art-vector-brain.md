# Answer — the brain endpoints did not intentionally ignore `Value::Vector`

**From:** Bee
**Date:** 2026-08-02
**Re:** your vec_probe question, same-day
**Source for every claim:** `github.com/nurdymuny/gigi` branch `vector-brain-gap`

## The answer: B. It was a gap. It is now fixed on a branch.

Your reading of the code was exactly right, arm for arm. `extract_field_samples`
matched `Float`/`Integer` only; `Value::Vector` fell to `_`; the skip-and-log
hardening dropped every record; the brain answered over zero samples with a 200.
Nothing in any commit message, comment, doc, or design note says vectors were
excluded on purpose — the extractor was written against Marcella's production
reality (exploded scalars, `v0..v383`), INGEST grew first-class Vector fibers on
a separate track, and the two surfaces never met. Your instinct that the fix
already existed in `dials.rs` was also right: the patch is that expansion,
lifted into the extractor.

**What changed** (branch `vector-brain-gap`, `src/stream_shared.rs`):

1. `extract_field_samples` now expands a `vector(d)` fiber into `d` sample
   columns per record — one requested name, `d` columns, mixed freely with
   scalar fields. Your 1-field schema works.
2. A vector whose length disagrees with its declared dims is treated as a
   poisoned row — skipped and logged, never zero-padded into the statistics.
3. **Your acceptance criterion is now an invariant:** if every record is
   skipped, the extractor returns an error naming the count and the offending
   field instead of `Ok(empty)`. No brain endpoint can answer confidently over
   nothing. An empty bundle still returns an honest zero — zero records is not
   blindness. Partial corruption keeps the fail-open contract (Hallie's ask #7
   is untouched; her original test still passes).

Four red-first tests pin all of it, including your vec_probe reproduction as a
unit test (`extract_field_samples_expands_vector_fibers`). Docs updated in the
same commit: `GQL_REFERENCE.md` INGEST note now states the expansion behavior
and dates the old symptom; the consumer-guide failure table has the row you
drafted, adjusted to the new behavior.

**For your ingestion decision — corrected 2026-08-02, read this over the
paragraph it replaces.** Build on **exploded scalars** (`v0..v383`). Your
default was right and my first answer over-promised.

An internal audit run immediately after this fix found the patch reached the
extractor but not the whole path. Two more layers assumed one column per field
NAME: the matrix builder computed width as `fields.len()`, so a `vector(384)`
fiber built an n×1 matrix over an n×384 buffer — KDE and nearest-record
results computed over reinterpreted memory, with a healthy `n_samples`. That
one was mine, introduced by the fix itself; it is now fixed (`77a81b8`) with
the width taken from the data and a ragged-row guard, and the four brain
handlers now validate `query` against a schema-aware width (1 per scalar,
`dims` per vector) instead of the field-name count.

The third layer is the honest boundary and it is **not** closed: bundle
Welford statistics do not track vector fibers, so an end-to-end probe of
`/brain/confidence` on a `vector(4)` bundle returns

    {"error":"no Welford stats for field 'emb'. Available stats: [] ..."}

A named refusal rather than a confident number — your acceptance criterion
holds — but vector fibers do **not** reach the brain endpoints yet, and I am
not going to call that first-class. Exploded scalars are the supported
representation today, on every build. When the statistics layer learns vector
fibers, the 1-field schema becomes real and you can migrate if you want to;
`INGEST` accepts both shapes, so nothing you write now is wasted.

## Your smaller questions, verified answers

**1. Record-returning k-NN over `Value::Vector`: no.** And you found a
documentation debt while asking: `SIMILAR` has a worked section in
`GQL_REFERENCE.md` but no token in the parser — it is documented and not
implemented, which violates our own enforced-table discipline; it gets the
stddev treatment (clearly marked or removed). What exists: the dials' `locus=`
computes statistics over the k-nearest by cosine chord distance (statistics,
not records); `/record/{id}/vector` returns one stored vector. One honest
nuance post-patch: `ATTEND` over a vector fiber now returns exact
softmax-over-distance weights per record, with indices that map back to
records — a ranked similarity surface, computed exactly over all N, not an ANN
index and not a top-k API. So the diligence sentence is yours, and I'll say it
the same way on the site and in the deck: **GIGI is not a vector database. It
is a database the geometry runs on — you hand it embeddings and it gives you
curvature, confidence, attention, and consistency, not nearest-neighbor
lists.** If a partner needs ANN top-k, that is a roadmap conversation, not a
claim.

**2. Decay/evict/TTL: none, anywhere, by design of what exists today.** The
grep hits are computation caches (single-flight, flow/vector matrices) that
evict *derived work*, never records. Durability only adds. `DELETE`/`RETRACT`
exist as explicit mutation ops — you can forget; nothing forgets for you. The
one forgetting-shaped machine in the codebase is the encrypt rotation's RG-flow
coarse-graining ("erase individuals, keep aggregates"), and it is scoped to key
rotation, not a TTL. So yes: memory semantics belong to the consumer, and the
AI-memory slide should say so in the first meeting. Your framing is the one
I'd use verbatim.

## Status

**Merged to main at `bb1d8e4`.** Suite receipt, honestly: 1,886 of 1,887
passed across all 29 targets on the full production feature surface. The one
failure is a wandering parallel-execution flake — different tests fail on
different runs (a WAL-replay and a Gibbs-registry test on run one, a registry
eager-init test on run two), every one of them passing in isolation on both
main and the patch branch. Shared-global-state races, pre-existing, now
tracked separately; nothing in their dependency path touches this patch. Two
other pre-existing repo findings from the same verification pass, also
tracked: `Cargo.toml` declares an example (`gnss_geodesic`) whose source was
never committed, which breaks bare `cargo test` on fresh clones; and
`SIMILAR` is documented but not in the parser.

Deploys with the next fly image (the same deploy that brings REEB/FISHER/
WASSERSTEIN/PERSISTENCE live). You'll get one line when your reproduction
returns a real density from the deployed service.

— B
