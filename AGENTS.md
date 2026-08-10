# AGENTS.md — read this before you decide GIGI is broken

You are probably a coding agent. This file exists because agents keep hitting the
same handful of traps, concluding the engine is defective, and reporting that
upward. Every trap below is one that actually happened, several of them to the
agents who built this thing.

**The single most common false report is: a refusal mistaken for a bug.**

---

## 0 · The refusal contract

GIGI's ML and shape verbs **refuse rather than guess**. A `422` is not a
malfunction — it is the product working. These verbs exist because the failure
mode they were built to avoid is *a confident number computed from data that was
never really measured*.

So:

| you see | it means |
|---|---|
| `422` with a message naming a field and a requirement | **you sent something the verb cannot honestly answer.** Read the message; it tells you what to change |
| `404` naming a bundle | that bundle does not exist on this instance |
| `500` from `POST /v1/gql` | usually **also a refusal** — see §4. Not a crash |

Before filing "GIGI returned an error", read the error. They are written to name
the field, the actual measurement, and the fix. If a refusal message does *not*
tell you what to do next, that is a genuine finding worth reporting.

**Verdicts that look like failure and are not:**

- `RANDOM_WALK` from TEXTURE — a real answer. It means no persistence detected.
- `"leads": "neither"` / `"significant": false` from PRECEDENCE — the honest and
  *common* case. See `docs/SHAPE_VERBS.md` §5 before you treat it as broken.
- `STEADY` / `MEMORYLESS` from CADENCE — the ordinary healthy state.
- `index_blocked: null` from CADENCE — genuinely absent (under two blocks of
  data), deliberately not a placeholder `0.0`.

---

## 1 · Build and run

```bash
cargo build --release --bin gigi-stream
PORT=3141 GIGI_DATA_DIR=/some/scratch/dir ./target/release/gigi-stream.exe
```

**Use `127.0.0.1`, never `localhost`.** On Windows `localhost` resolves `::1`
first and costs about **2 seconds per request** against 11 ms on the loopback
address. An agent that uses `localhost` will conclude the engine is slow.

**The stale-binary trap — this one has bitten repeatedly.** `cargo test` rebuilds
the *library*. It does **not** rebuild or restart a running `gigi-stream`. So:

- a green test suite says nothing about the HTTP behaviour of an engine you
  started earlier;
- if you edit code and re-probe a running server, **you are testing the old
  binary**. Rebuild and restart.

**`Access is denied. (os error 5)` on build** means a running `gigi-stream.exe`
is holding the output file. Cargo reports this *without failing loudly enough to
notice* — the link step silently does not happen and you keep running the old
binary. Kill the engine, then rebuild:

```bash
taskkill //FI "IMAGENAME eq gigi-stream.exe" //F
```

Always confirm a rebuild actually relinked (check the binary's mtime) before
concluding a fix did not work.

---

## 2 · Verify against a *live* engine, not just tests

A green `cargo test` is necessary and not sufficient. Several real defects in
this repo were invisible to the suite and obvious on the first live call —
including a verb that returned HTTP 200 having executed nothing.

For anything customer-facing, run it against a running instance on data with a
**planted answer** you chose in advance. Do not eyeball plausible-looking output
and call it correct.

---

## 3 · `POST /v1/gql` can return success having done nothing

If `/v1/gql` cannot bind a statement to a bundle, it returns:

```json
{"status": "ok"}
```

**having executed nothing.** HTTP 200. This is a known sharp edge in the
dispatcher, not a sign your query worked. If you get `{"status":"ok"}` from a
statement you expected to return data, your verb is not wired into
`get_bundle_name` — that is a real bug, but the symptom looks like success.

A verb that *did* run returns rows or a value:

```json
{"value": 0.064}                       // scalar verbs, e.g. CURVATURE
{"rows": [ { ... } ]}                  // row-returning verbs
```

---

## 4 · GQL errors come back as 500, REST as 422

The same refusal has a different status code depending on the surface:

```
POST /v1/bundles/b/texture   {"field":"nope"}   -> 422  {"error": "field 'nope' not found"}
POST /v1/gql  {"query":"TEXTURE b ON nope;"}    -> 500  {"error": "field 'nope' not found"}
```

`/v1/gql` maps every execution error to 500, for every verb. **Do not read that
as a server crash, and do not retry it** — it is a permanent input error. The
message is identical to what REST gives you.

---

## 5 · Parameter names that bite

Getting these wrong produces a confusing result rather than an error, which is
how they turn into false bug reports.

| verb | correct | commonly guessed wrong |
|---|---|---|
| `/cluster` | `k` | ~~`n_clusters`~~ |
| `/cadence` | `time` | ~~`order`~~ — see below, this one is load-bearing |
| `/texture` | `field`, `order` | — |
| `/precedence` | `x`, `y`, `order` | — |

**CADENCE takes `time`, not `order`, and the difference is not cosmetic.**
TEXTURE and PRECEDENCE take an *ordinal* sort key whose values are ignored.
CADENCE needs a *cardinal* clock — the values **are** the measurement. Passing a
row index gives perfectly uniform gaps, which is mathematically `index = 0`, a
maximally damning "your feed is metered" verdict on healthy data. The verb
refuses that input by name, but only because someone got it wrong first.

Unknown request keys are currently **ignored**, not rejected — and that is how a
typo becomes a wrong answer rather than an error. Measured on one bundle:

```
{"field":"v","order":"seq","max_lag":64}   -> 200, exponent 0.585823
{"field":"v","order":"seq","maxLag":64}    -> 200, exponent 0.502294   <- typo
{"field":"v","order":"seq"}                -> 200, exponent 0.502294   <- the default
```

The camelCase spelling silently fell back to the default and returned a
materially different number with no complaint. **If a parameter seems to have no
effect, check its spelling before concluding it is unimplemented.**

---

## 6 · Record order and storage mode

TEXTURE and PRECEDENCE read **record order**. That equals insertion order only
on *sequentially stored* bundles. **A bundle with a TEXT base field is
hash-stored and iterates arbitrarily.**

Omitting `order` on such a bundle is now **refused** (422 naming the storage
mode). Name an ordering field:

```sql
TEXTURE btc ON mid ALONG seq;
```

If you are writing test fixtures: a schema with a categorical base field is
hash-stored, so your fixtures need an explicit ordering field too. This caught
the repo's own tests.

---

## 7 · Reading a result honestly

Every verb returns its own uncertainty. Use it.

| verb | the number | the ruler beside it |
|---|---|---|
| TEXTURE | `exponent` | `r_squared` (is it even self-similar?) and `h_sd` |
| PRECEDENCE | `area` | `p_value`, `null_sd`, `significant` |
| CADENCE | `index`, `memory` | `index_z`, `memory_z`, `null_sd` |

**Do not report a bare estimate.** In particular, PRECEDENCE's `area` cannot on
its own distinguish a real lead from noise — the statistic does not concentrate
with more data, and its noise floor varies by a factor of ~160 with how rough
the data is. A single window that reports `significant: false` has not failed;
it has told you the truth. `docs/SHAPE_VERBS.md` §5 and §7 explain the
aggregation pattern that gives the verb its power.

Every response also carries a `reads` array of plain-English sentences and a
`notes` array of disclosures (rows skipped, sorting applied, sample capped).
**Read `notes` before concluding a number is wrong** — it usually explains it.

---

## 8 · Where the real documentation is

| you want | read |
|---|---|
| the three shape verbs, end to end | `docs/SHAPE_VERBS.md` |
| first bundle, from scratch | `docs/GETTING_STARTED.md` |
| building an agent/LLM on GIGI | `docs/CONSUMER_PATTERNS.md` |
| every ML endpoint, machine-readable | `GET /v1/ml` on a running instance |
| full route catalog | `GET /v1/openapi.json` |
| what shipped when, and why | `CHANGELOG.md` |
| feature-flag stability tiers | `docs/STABILITY_GUARANTEES.md` |

`GET /v1/ml` is the fastest way to discover what exists and what parameters it
takes. Prefer it over guessing from source.

---

## 9 · If you are changing code here

- **Gates must fail when the mechanism is removed.** Before writing "this test
  proves X", delete X from the fixture and confirm the test goes red. A gate
  that passes on a broken implementation is worse than none. This has caught
  real defects in this repo more than once.
- **A gate that accepts two outcomes tests neither.** If two results are both
  acceptable, that is two fixtures, not one assertion with an `or`.
- **Never assert a sign or a direction — plant it and measure.** The PRECEDENCE
  sign convention was written down wrong twice by people reasoning about what
  felt right. The estimator was correct both times.
- **Bash heredocs on this Windows setup eat backslashes**, even quoted. Writing
  Rust string continuations or regexes through a heredoc will silently mangle
  them. Use the file-writing tool instead.
- Run `cargo test` (all targets, not just `--lib`) before claiming green. The
  route-drift gate is an integration test and `--lib` does not run it.

---

## 10 · Before you report "GIGI is broken"

Check, in order:

1. Did you read the error message?
2. Are you on `127.0.0.1`?
3. Is the running binary actually the one you just built?
4. Did `{"status":"ok"}` mean success, or nothing-executed (§3)?
5. Is a `500` from `/v1/gql` actually a refusal (§4)?
6. Are your parameter names right (§5)?
7. Did you name an ordering field (§6)?
8. Does `notes` already explain the result (§7)?

If it still looks wrong after all eight — it may well be. Report it with the
exact request, the exact response, and what you expected instead. That report is
welcome and useful. The ones that waste everybody's time are the seven above.
