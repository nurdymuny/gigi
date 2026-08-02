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

**For your ingestion decision:** both representations are now first-class.
`emb: vector(384)` in one field, or exploded `v0..v383` — the brain sees the
same matrix either way. If you want zero dependency on this branch landing in
the deployed image before your sprint, exploded scalars work on every build
that exists; if you take the 1-field schema, wait for the deploy note. Either
way your ~790 records do not need a migration later — INGEST accepts both.

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

Branch `vector-brain-gap`: extractor patch + 4 tests + doc fixes. Full
production-surface suite running now; merges to main when it's green, deploys
with the next fly image (same deploy that brings REEB/FISHER/WASSERSTEIN/
PERSISTENCE live). You'll get one line when your reproduction returns a real
density from the deployed service.

— B
