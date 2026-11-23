# GIGI — Internal Security Review

**Baseline:** `ae6e004` (main, 2026-08-02)
**Target:** `src/bin/gigi_stream.rs` and the engine/crypto modules it links, as deployed at `gigi-stream.fly.dev`
**Method:** source review of the full HTTP surface by five scoped passes, followed by a verification pass that attempted to *refute* each claim. Most findings were exercised end-to-end against locally-run instances of the real release binary (`target/release/gigi-stream.exe`) with the production feature set. No file in the repository was modified. No request was made to the production service.

---

## What this review is not

This is the owner's internal code review of her own engine, run before external diligence so that the weaknesses are found by us first. It is **not** an independent third-party audit, and it is **not** a cryptanalysis of the geometric gauge scheme — that remains a planned deliverable per `GIGI_ENCRYPTION_SPEC.md`, and nothing here should be read as substituting for it. The crypto findings below are *implementation* defects (key persistence, KDF construction, error handling) that hold regardless of whether the underlying scheme is sound.

The review did **not** include fuzzing, property testing, or a systematic runtime test campaign. Confirmations were targeted reproductions of specific hypotheses, not a search of the input space. Absence of a finding in a given area is not evidence that the area is clean — see the five coverage notes at the end, which state exactly what each pass reached and what it did not.

Two categories were deliberately excluded as already-disclosed design properties rather than defects: affine-mode order preservation (disclosed at `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md:119`) and the general "validation to date is mathematical" caveat in the encryption spec.

## How to read this

Findings are ordered by severity, CONFIRMED before UNVERIFIED, and each carries the attack path, impact, evidence with `file:line`, and a fix sketch. Severity follows the rubric this review was run under:

- **HIGH** — unauthorized read/write, key/secret exposure, or a trivially-triggered service kill.
- **MEDIUM** — degradation, resource exhaustion requiring volume, or an information leak.
- **LOW** — hardening; correct to fix, no traced path to a bad outcome on its own.

Where the verification pass changed a severity, that is marked inline. Where two passes disagreed, both arguments are recorded rather than silently resolved.

**One precondition runs through several findings.** The tenant-authorization findings (H4, H5, H6, M1, M2) require a valid non-owner JWT. `GIGI_JWT_SECRET` is not in `fly.toml`'s `[env]` block and would be set via `flyctl secrets`, which is not observable from the repository. If it is unset, `state.jwt_secret` is `None` (`src/bin/gigi_stream.rs:334`) and no non-owner token can exist today — the gaps are then latent rather than live. The token mint is documented as active (`src/bin/gigi_stream.rs:60-77`, `docs/HTTP_API_REFERENCE.md:40-43`), and the gaps are structural either way, so they are reported as findings. **Run `flyctl config env` before deciding urgency on those five.**

### Counts

| Severity | Confirmed |
|---|---|
| HIGH | 11 |
| MEDIUM | 8 |
| LOW | 14 |
| **Total confirmed** | **33** |
| Unverified leads | 10 |

Three findings are reachable with **no credentials of any kind** (H1, H2, H3). Two of those kill the process.

---

# CONFIRMED FINDINGS

## HIGH

---

### H1 — EXISTS subquery inside an allowlisted COVER reads any bundle (anonymous)

`src/bin/gigi_stream.rs:12566`, executor at `:13615`

**Attack path.** `POST /v1/public/gql` is anonymous: `auth_middleware` matches the path, stamps `GigiClaims::owner_via_api_key()` and returns without authenticating (`src/bin/gigi_stream.rs:1373-1376`). `namespace_enforcement_middleware` is a no-op because `parse_bundle_segment("/v1/public/gql")` hits the `_ => None` arm (`:1554`). The single-statement guard at `:12628` passes because there is no `;`. `validate_public_stmt` then matches `S::Cover { bundle, .. } => bundle_ok(bundle)` — the `..` discards `where_conditions`, and `FilterCondition::Exists` carries a **second, unchecked bundle name** parsed at `src/parser.rs:6733-6748`. `FilterCondition::Exists` returns `None` from `field_name()` (`src/parser.rs:1804`), so the Cover arm's schema-validation loop skips it too. `filter_to_query_conditions` maps `Exists` to `vec![]` (`src/parser.rs:8636`), so the outer predicate vanishes and the outer COVER runs unfiltered. `execute_gql_with_exists` then resolves the inner name against the whole engine with no allowlist and no claims.

**Verified live.** With `GIGI_PUBLIC_BUNDLES="stations"`, anonymous `COVER secret_kv ALL` was correctly refused with `bundle 'secret_kv' is not exposed on the public read endpoint`. Anonymous `COVER stations ALL WHERE EXISTS (COVER secret_kv WHERE payload CONTAINS 'TOPSECRET')` returned all rows (true); the same query with `'NOPE'` returned `count:0` (false). A 64-character-alphabet `MATCHES '^<prefix>'` walk reconstructed the private plaintext exactly. `EXISTS (COVER _gigi_query_log)` returned true and `EXISTS (COVER no_such_bundle)` returned false — a working private-bundle-name enumerator. A range oracle (`salary > 249999` true, `> 250000` false) recovered an exact private numeric value in four requests.

**Impact.** An anonymous internet caller reads private data out of **every** bundle in the engine, not just the four demo bundles. Concrete production targets from `fly.toml`'s own `GIGI_APP_BUNDLES`: `jg_kv`'s `key`, `kind`, `expires_at`, `updated_at` are declared plaintext (only `payload` is opaque/AEAD), so chat KV keys and metadata are fully recoverable; every other bundle on the box (`marcella`, `halcyon`, `claude_substrate_v0`, the `_gigi_*` system logs) yields existence, schema shape, row counts, and — via `MATCHES`/`CONTAINS`/range predicates — field contents character by character. This is precisely the guarantee the endpoint exists to hold. Unmetered: `GIGI_RATE_LIMIT` is unset, so `rate_limit_middleware` short-circuits at `:1580`. The test block at `:21243-21340` never exercises a nested-bundle reference, so it passes green.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:12566 — validator, `..` swallows where_conditions
        S::Cover { bundle, .. } => bundle_ok(bundle),

// src/bin/gigi_stream.rs:13615-13620 — executor, no allowlist in scope
    if let FilterCondition::Exists { cover_bundle, where_conds } = fc {
        if let Some(sub_store) = engine_read.bundle(cover_bundle) {
            let sub_qcs: Vec<gigi::bundle::QueryCondition> = where_conds.iter()
                .flat_map(gigi::parser::filter_to_query_conditions)
                .collect();
            !sub_store.filtered_query_ex(&sub_qcs, None, None, false, Some(1), None).is_empty()
```

**Fix sketch.** Make `validate_public_stmt` structural rather than shallow: walk every `FilterCondition` in `on_conditions`, `where_conditions` and `or_groups` recursively (an `Exists` can nest another `Exists`) and run `bundle_ok` on each `cover_bundle`. Destructure `Cover` explicitly instead of `..`, so a new bundle-carrying field cannot be added silently — the same `..` also swallows `excluding` (see U1). Belt-and-braces, and the more durable half: thread `state.public_bundles` into the executor and make every `engine.bundle()` lookup on the public path go through an allowlist-checked accessor, so the check lives next to the engine lookup that actually crosses the trust boundary rather than in a validator that must be kept in sync with the grammar.

---

### H2 — Unbounded parser recursion aborts the process (anonymous, one request)

`src/parser.rs:6743`, `:5714`, `:8759`

**Attack path.** `gigi::parser::parse()` runs at `src/bin/gigi_stream.rs:12643`, **before** `validate_public_stmt` at `:12654`. Consequently the *entire* grammar — all 140+ productions, not just the ten allowlisted read verbs — is reachable pre-auth on the anonymous public endpoint. Three productions recurse with no depth counter:

- `parse_filter_condition_list` → itself, once per nested `EXISTS` (`src/parser.rs:6743`)
- `parse_invariant_term` → `parse_invariant_expr`, once per `(` (`src/parser.rs:5714`)
- `parse_weight_atom` → `parse_weight_add_sub`, once per `(` (`src/parser.rs:8759`; live because `Dockerfile:11` builds with `patterns`)

**Verified live** against the release binary, twice, independently. `COVER demo WHERE ` + `EXISTS (COVER demo WHERE ` ×10000 + … (260 KB body) → no response, server log `thread 'tokio-rt-worker' has overflowed its stack`, `Get-Process gigi-stream` count 0. Health went 200 → connection-refused. Separately, `PROJECT INVARIANT (` + `(`×30000 + `curvature` + `)`×30000 + `) FROM demo;` (60 KB) posted to `/v1/public/gql` **with no credentials** produced the same abort. The second reviewer had assumed PROJECT INVARIANT required a key because it is not on the verb allowlist; that assumption is wrong, and the correction runs in the attacker's favour.

**Impact.** Full process death, not a caught panic. A Rust stack-guard overflow calls `abort()` — unwinding never runs, so no handler can intercept it, and there is no panic layer anyway (`grep CatchPanicLayer` over `src/` returns nothing; `tower-http` is compiled with `features = ["cors"]` only, `Cargo.toml:143`). GIGI is single-node holding bundles in heap + mmap, so every in-flight request dies and the machine cold-restarts into WAL replay. `fly.toml` sets `restart_policy = "always"`, but a single cheap unauthenticated POST re-fires faster than a cold start plus replay, so the service can be held down indefinitely by an attacker with no credentials.

Four refutation attempts all failed: ordering (parse genuinely precedes validation), auth (`:1373` bypasses unconditionally), body limit (no `DefaultBodyLimit` or `RequestBodyLimitLayer` anywhere in `src/`, so axum's 2 MB default applies and 260 KB is well under it), panic catcher (none compiled, and irrelevant to an abort). No depth counter exists: every `depth` hit in `src/parser.rs` is a paren/token balance counter (`:7128`, `:7348`, `:7492`) or unrelated.

**Evidence.**
```rust
// src/parser.rs:6738-6748 — no depth bound
                self.expect_keyword("COVER")?;
                let cover_bundle = self.expect_word()?;
                let where_conds = if self.is_keyword("WHERE") {
                    self.advance();
                    self.parse_filter_condition_list()?   // <-- :6743

// src/parser.rs:5711-5716 — same shape, different production
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let inner = self.parse_invariant_expr()?;     // <-- :5714
```

**Fix sketch.** Add a `depth: usize` field to the `Parser` struct, increment on entry to every recursive production, and bail with a parse error past a fixed ceiling (64 is generous for real queries). Do it on the struct rather than per-site so one guard covers all three known productions at once — then grep for every self-recursive or mutually-recursive `parse_*` pair and confirm the guard reaches them, because a single missed production leaves the abort live. The guard must sit **before** verb dispatch: a verb allowlist provably does not contain this. Independently and as defence in depth, cap request size on `/v1/public/gql` far below 2 MB (a few KB suffices for a read verb) and cap token count in `tokenize()`.

---

### H3 — Unbounded attacker-keyed regex cache (anonymous memory exhaustion)

`src/bundle.rs:210` — *upgraded from MEDIUM during verification*

**Attack path.** `COVER stations ALL WHERE name MATCHES '<pattern>'` on the anonymous public endpoint. `MATCHES` parses to `FilterCondition::Matches` (`src/parser.rs:6779-6787`), maps to `QueryCondition::Regex` (`src/parser.rs:8617`), and `Cover` passes the allowlist because `stations` is exposed. `QueryCondition::matches` then hits a `thread_local` `REGEX_CACHE` — an unbounded `HashMap` keyed on the **caller's pattern string**, with `or_insert_with`, no eviction, and no `RegexBuilder` size limit. Each distinct pattern permanently retains a compiled `Regex` on that worker thread.

**Verified live, and materially worse than first reported.** 200 anonymous requests with distinct `(?:a{200}){200}x<i>` patterns grew RSS from 21.8 MB to 410 MB; 200 more took it to 797 MB — monotonic, no release between rounds, ~1.94 MB permanently retained per distinct pattern. The amplification the first pass missed: AND-chained conditions short-circuit, so 50 *non-matching* MATCHES conditions compile only the first. Patterns written to **match** — `name MATCHES 'alpha|(?:a{200}){200}q<i>'` — let evaluation proceed through every condition. One 2.6 KB anonymous request carrying 50 such conditions parked **162 MB** of permanent RSS. At that rate roughly 200 requests exhaust the 32 GB Fly VM, and the 2 MB body limit allows far more than 50 conditions per request.

**Impact.** Anonymous, unmetered, few-hundred-request kill of a single-node engine that holds the only copy of live state — a trivially-triggered service kill by the stated rubric. The cache is per-thread, so the leak is invisible in any per-bundle metric. The authenticated `/v1/gql` path is affected identically.

**Evidence.**
```rust
// src/bundle.rs:208-219
                        thread_local! {
                            static REGEX_CACHE: std::cell::RefCell<HashMap<String, Option<Regex>>> =
                                std::cell::RefCell::new(HashMap::new());
                        }
                        REGEX_CACHE.with(|cache| {
                            let mut cache = cache.borrow_mut();
                            let compiled = cache
                                .entry(pattern.clone())
                                .or_insert_with(|| Regex::new(pattern).ok());
```

**Fix sketch.** Bound the cache with a small LRU (128 entries is ample) and build through `RegexBuilder` with explicit `size_limit` and `dfa_size_limit` well under the ~10 MiB default. Cap pattern length and the number of `MATCHES` conditions per statement at parse time. Set `GIGI_RATE_LIMIT` (M4) — rate limiting is compiled in but disabled in production, which is what makes this practical rather than theoretical.

---

### H4 — `POST /v1/gql` performs no tenant authorization

`src/bin/gigi_stream.rs:12679` (route registered `:16338`)

**Attack path.** A non-owner tenant token (`{email, ns:"ns_<12hex>", owner:false, exp}`) authenticates at `:1441-1456`, and `GigiClaims{owner:false, ns:"ns_abc"}` is stashed. `namespace_enforcement_middleware` gates only what `parse_bundle_segment` extracts from the URL **path**; for `/v1/gql` the second segment is `gql`, which matches neither `bundles` nor `ws`, so it hits `_ => None` at `:1554` and the middleware returns without ever calling `allows_bundle`. `gql_query`'s signature is `(State, HeaderMap, Json<Value>)` — it never reads request extensions. A grep for `GigiClaims` across the 21.5k-line file returns hits at exactly four sites: `:1509` (middleware), `:1853` (`list_bundles`), `:1880`/`:1913` (`create_bundle`). Nothing on the GQL path.

**Verified live.** With a `GIGI_JWT_SECRET`-signed `owner:false` token: `GET /v1/bundles/secret_kv/records` was correctly 403'd by the namespace middleware — proving the token is genuinely non-owner and the path gate works. The **same token**, via `POST /v1/gql`: `SHOW BUNDLES;` listed all 12 bundles with record and field counts including every `_gigi_*` system bundle; `COVER secret_kv ALL` returned the full record `{"kind":"session","payload":"TOPSECRET","key":"sess:abc123"}`; `INSERT INTO secret_kv …` succeeded and read back; `COLLAPSE secret_kv;` returned ok and the bundle was gone.

**Impact.** Complete failure of the multi-tenant model for any tenant who sends one request to a different URL: cross-tenant read of all records (decrypted — `src/bundle.rs:2740-2748` and `:2144-2151` decrypt fiber values transparently on every read), cross-tenant write, and irreversible cross-tenant bundle deletion. `COLLAPSE` at `:12841` calls `engine.drop_bundle` unconditionally with no `_gigi_` guard — the guard exists at exactly three REST sites (`:1998`, `:2049`, `:17073`) and nowhere in the executor — so `_gigi_audit_log` can be destroyed from this endpoint. The contrast is deliberate elsewhere: `list_bundles` filters by claims and `public_gql_query` filters `ShowBundles` at `:12664`, while the `/v1/gql` `ShowBundles` arm at `:12825` enumerates `engine.bundle_names()` unfiltered. The same shape applies to every route whose bundle name lives in the body rather than the path: `/v1/divergence`, `/v1/quantum_cohomology/*`, `/v1/wish`, `/v1/transactions/*`, `/v1/lattice`, `/v1/gauge_field`.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:1535-1543 — the only gate, path-shaped
    match parts.next()? {
        "bundles" => { let name = parts.next()?; ... }
        "ws"      => { ... }
        _ => None,
    }

// src/bin/gigi_stream.rs:12679-12683 — no claims anywhere in the signature
async fn gql_query(
    State(state): State<Arc<StreamState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
```

**Fix sketch.** Authorize on the **statement**, not the URL. Add `get_bundle_names(&Statement) -> Vec<&str>` returning every bundle a statement touches — primary target, `Pullback` left+right, `Join` left+right, `Exists.cover_bundle` recursively, `EXCLUDING IN` names, `Emit` inner — and require `claims.allows_bundle(n)` for all of them before dispatch, including the pre-dispatch gauge/topology/ingest blocks. Note `get_bundle_name` at `:15233` is not sufficient as written: it returns only `left` for Pullback/Join and nothing for CreateBundle/Collapse. Filter the `ShowBundles` arm by claims the way `public_gql_query` already does. Extend the `_gigi_*` deny-list to the executor's Collapse/Retract paths. Then repeat the statement-level check on every body-carries-the-bundle-name route.

---

### H5 — WebSocket `/ws` performs no per-message authorization

`src/bin/gigi_stream.rs:12100` (handler `:11842`, route `:16429`)

**Attack path.** A client connects to `wss://host/ws` with `Sec-WebSocket-Protocol: gigi.v1, gigi.bearer.<token>` (`extract_subprotocol_credentials`, `:1330`); `auth_middleware` accepts it. `/ws` has no `/v1` prefix, so `parse_bundle_segment` returns `None` on the `first != "v1"` check (`:1532`) and namespace enforcement never runs. `ws_handler` (`:11826-11840`) passes only `state` into `handle_ws` — the claims are present in request extensions at upgrade time and are simply never captured, and `handle_ws_command` (`:12046`) has no claims parameter. The SUBSCRIBE arm's only gate is a bundle-exists check (`:12096-12101`) before `get_or_create_channel` / `subscribe`.

**Impact.** Three distinct outcomes over one socket:
1. **Live cross-tenant exfiltration in plaintext.** The REST insert broadcast at `:2190-2200` sends `record_to_json(rec)` over the caller-supplied **pre-encryption** values, so `payload` — the field `fly.toml` declares `encrypted:"opaque"` precisely because chat content is the sensitive payload — is broadcast in cleartext and framed verbatim to the subscriber at `:11929-11934`.
2. **Cross-tenant point-read.** The QUERY arm (`:12190-12225`) calls `engine.bundle(bundle_name)` + `point_query` with no check — a direct read, not just a tap.
3. **Cross-tenant and system-bundle writes.** The INSERT arm (`:12134-12188`) calls `engine.bundle_mut(bundle_name)` with no namespace check and no `_gigi_` guard, so rows can be forged into `_gigi_audit_log`.

The bundle-not-found response also doubles as an existence oracle for private names. `/v1/ws/dashboard` (`:11770`) is ungated the same way, but `DashboardEvent` (`:290-315`) carries only aggregate curvature/anomaly metadata, so that one is metadata-only.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:12095-12104 — SUBSCRIBE arm, the entire check
            // Verify bundle exists
            {
                let engine = state.engine_read();
                if engine.bundle(&bundle_name).is_none() {
                    return format!("ERROR: Bundle '{}' not found", bundle_name);
                }
            }
            let tx = state.get_or_create_channel(&bundle_name);
            let receiver = tx.subscribe();
```

**Fix sketch.** Capture `GigiClaims` in `ws_handler` (already in request extensions at upgrade) and thread them into `handle_ws` / `handle_ws_command`; gate SUBSCRIBE, INSERT, QUERY and UNSUBSCRIBE on `claims.allows_bundle()`. Return the same generic error for not-found and not-authorized so the socket is not an existence oracle. Apply the `_gigi_` read-only guard to the WS INSERT arm. Separately, decide whether the broadcast should carry pre- or post-encryption values — broadcasting plaintext for a field declared opaque is a boundary violation even for an authorized subscriber.

---

### H6 — Cross-tenant record dump via unchecked `right_bundle` in `/join`

`src/bin/gigi_stream.rs:2624`

**Attack path.** A non-owner tenant POSTs to `/v1/bundles/ns_abc__scratch/join` — a bundle inside their own namespace, so the path gate passes honestly at `:1510`. The body carries `{"right_bundle":"jg_kv", "left_field":"id", "right_field":"key"}`. The middleware has already run and only ever saw the path segment; `pullback_join` resolves `req.right_bundle` at `:2624` with no claims consultation. The response serializes the **full right-hand Record** for every matched pair via `record_to_json` at `:2644-2647` — not a boolean. Because the tenant owns the left bundle, they seed it with whatever join keys they choose to force matches.

**Verified live.** The non-owner tenant created `ns_abc123def456__scratch` (allowed — `create_bundle` does check claims at `:1880`), inserted a row keyed `sess:abc123`, and POSTed the join. Response: `{"data":[{"left":{...},"right":{"key":"sess:abc123","kind":"session","payload":"TOPSECRET"}}]}`.

**Impact.** Direct, non-oracle exfiltration of another tenant's records by an authenticated non-owner. Unlike H4 this survives a fix to `/v1/gql`, because it is a plain REST route whose path segment passes the namespace gate honestly while the second bundle rides in the body. One reliability caveat that narrows the exploit but not the defect: the join materializes only when both sides are heap-backed (`match (left.as_heap(), right.as_heap())` at `:2631`; `BundleRef::as_heap` returns `None` for `Overlay`, `src/mmap_bundle.rs:1810`), so a victim bundle restored from a `.dhoom` snapshot on an mmap fast boot yields an empty result. Heap-backed victim bundles remain routine — anything created or schema-declared since the last snapshot, plus bundles degraded to heap-only by the graceful-skip path at `src/engine.rs:680`. `/v1/divergence` (`:9063`, `:9069`) has the identical body-bundle gap with **no** `as_heap` gate at all, and returned a KL/JS report across two attacker-named bundles.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:2624-2631
    let right = engine.bundle(&req.right_bundle).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Bundle '{}' not found", req.right_bundle),
            }),
        )
    })?;
```

**Fix sketch.** Pull `GigiClaims` from request extensions in every handler that resolves a bundle name from the body, and call `allows_bundle` before `engine.bundle()`. Structurally better: add a `state.bundle_for(&claims, name)` accessor that is the only way to obtain a `BundleRef` in the HTTP layer, and make `engine.bundle()` crate-private, so a newly added handler cannot forget.

---

### H7 — Integer overflow in `cluster`'s `k` check wedges the whole service

`src/ml/cluster.rs:352` (handler `src/bin/gigi_stream.rs:9370`, route `:16400`)

**Attack path.** One request from any authenticated tenant: `POST /v1/bundles/{name}/cluster` `{"method":"kmeans","k":9223372036854775808}`. `ClusterRequest.k` is a bare `usize` deserialized straight from JSON with no clamp (`src/ml/cluster.rs:19`). The `k<2` guard at `:328` passes. At `:352`, `2 * k` overflows — `Cargo.toml` has **no `[profile]` section at all**, so release keeps the default `overflow-checks = false` and `2 * 2^63` wraps to `0`, making `n < 0` false and the guard a no-op. `CLUSTER_MAX_N` at `:355` also passes. Control reaches `kmeans_lloyd`, whose k-means++ init loop `while cen.len() < k` (`:262`) needs 2^63 iterations. `bundle_cluster` takes `state.engine_read()` at `src/bin/gigi_stream.rs:9370` and holds the guard across `cluster_records` at `:9375-9379` with no `spawn_blocking`.

**Verified live.** The request never returned. 20 s later a SECTION write returned `000` and a plain COVER read returned `000` — `engine_write()` queues behind the held read guard, and std's `RwLock` is write-preferring, so new readers then starve too. `/v1/health` and anonymous `/v1/public/gql` both returned `000` at 15 s. The process was still alive having burned 84 CPU-seconds; it never self-clears.

**Impact.** Permanent, total loss of query service — reads and writes both — from a single request with no volume required, recoverable only by process restart. The same overflowed guard admits huge `k` to the gmm and spectral paths, and the unbounded `cen.push` leaks memory alongside. One correction to the original report, which does not change severity: `/v1/health` does *not* stay green throughout — it answers 200 from the `Ok` arm during the read-only phase and then stops responding entirely once blocked requests exhaust the tokio workers, at which point `fly.toml`'s 3 s readiness check fails. So the platform does eventually notice; the outage is real either way.

**Evidence.**
```rust
// src/ml/cluster.rs:352 — 2*k wraps to 0 in release
    if method != "dbscan" && n < 2 * k {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cluster needs at least 2·k = {} records to form {} clusters (bundle has {})", 2 * k, k, n)));
    }
```

**Fix sketch.** Clamp `k` to an absolute ceiling before any arithmetic (`k <= n` and `k <= 512`), and write the guard overflow-free as `k > n / 2` rather than `n < 2 * k`. Add `overflow-checks = true` to `[profile.release]` so the next such wrap panics loudly instead of computing a wrong bound. Then the part that turns a bad request into an outage: run `cluster_records` and the other CPU-bound ML entry points under `tokio::task::spawn_blocking`, and hold the engine read guard for the extraction phase only, not across the solve.

---

### H8 — Unbounded n² allocation from request-body `fields` length, before validation

`src/bin/gigi_stream.rs:7819` (entry `:7892`)

**Attack path.** Any of ten brain endpoints — `/v1/bundles/{b}/brain/{sample,dream,forecast,reconstruct,inpaint,predict,fit_diagnostics,distance_to_fit_mean,intent_gate,confidence_with_explain}`, call sites at `:5454`, `:5628`, `:5809`, `:6910`, `:7179`, `:8359`, `:8476`, `:8578`, `:8693`, `:8803`, all registered routes (e.g. `:16482`, `:16514`, `:16518`) — accepts a body `{"fields":[…50 000 one-char entries…]}`, ~200 KB. `flow_from_bundle_cached` takes `n = fields.len()` and calls `canonical_symplectic_pad(n)` as its **first** statement (`:7892-7894`) — before the cache lookup at `:7902` and before `compute_fit_data` at `:7946`, where field names are first checked against the schema. The only shape constraint is `n >= 2 && n % 2 == 0` (`:7815`). `TwoForm::new` then allocates a `dim × dim` array and **writes every cell** in a double loop (`src/geometry/forms.rs:96-110`), so the memory cannot stay on lazily-mapped zero pages. `kahler` is in the production build (`Dockerfile:11`).

**Impact.** At n = 50 000 that is 20 GB written on a 32 GB machine (`fly.toml memory = "32gb"`); at n ≈ 500 000 — still inside the 2 MB body limit — the layout is ~2 TB, allocation fails, and `handle_alloc_error` aborts. Process kill either way, from one ~200 KB request, repeatable with no rate limit. Amplification ratio is roughly 100 000×. (Correction to the original arithmetic that does not change the verdict: `raw` at `:7819` is `vec![0.0; n*n]`, i.e. `alloc_zeroed`, so the resident cost is the single array `TwoForm::new` writes, not two.)

**Evidence.**
```rust
// src/bin/gigi_stream.rs:7815-7819
fn canonical_symplectic_pad(n: usize) -> Option<gigi::geometry::ClosedTwoForm> {
    if n < 2 || n % 2 != 0 { return None; }
    let half = n / 2;
    let mut raw = vec![0.0_f64; n * n];

// src/bin/gigi_stream.rs:7892-7894 — first statement, ahead of all validation
    let n = fields.len();
    let b = canonical_symplectic_pad(n)
        .ok_or_else(|| bad_request("dimension must be ≥ 2 and even"))?;
```

**Fix sketch.** Cap `fields.len()` at the top of `flow_from_bundle_cached` (512 is generous) and reorder so schema validation of every field name runs before `canonical_symplectic_pad`. Cap `dim` inside `TwoForm::new` itself so no future caller can reintroduce it. Deduplicate `fields` — `extract_field_samples` (`src/stream_shared.rs:42-77`) currently accepts the same name repeated arbitrarily.

---

### H9 — Unbounded `n_steps` / `n_samples` / `burn_in` on brain flow endpoints

`src/geometry/generative_flow.rs:338` (and `:310`, `:277-283`, `:361`)

**Attack path.** `POST /v1/bundles/{b}/brain/dream` with `{"fields":["x","y"],"n_steps":2000000000}` — about 50 bytes. `brain_dream_endpoint` copies `req.n_steps` verbatim into `FlowConfig` at `src/bin/gigi_stream.rs:8378` and calls `dream`, which does `Vec::with_capacity(config.n_steps + 1)` of `Vec<f64>` (24 B each) at `generative_flow.rs:338` — 48 GB for the outer vec alone. `GenerativeFlow::validate` (`:368-386`) inspects only `initial.len()`, `dt`, and `temperature`; it never looks at `n_steps`, `n_samples`, or `burn_in`, and a grep for caps on those fields across the binary returns nothing (the only clamps in the file are on unrelated fields — `req.epochs.clamp(1,5000)` at `:9293`, `req.folds.clamp(2,20)` at `:9327`). Same shape on `/forecast` (`:310`), `/sample` (`:277-283`), `/reconstruct` (`:361`).

Two variants:
- **Overflow.** With no `[profile.release]` section in `Cargo.toml`, `n_steps: usize::MAX` makes `config.n_steps + 1` wrap to capacity 0, and `for _ in 0..config.n_steps` then runs 2^64 iterations pushing to an unbounded Vec — a hang ending in OOM.
- **Pure spin.** `burn_in` allocates nothing, so `{"burn_in":100000000000}` is an infinite CPU spin (`generative_flow.rs:277`, fed from `src/bin/gigi_stream.rs:5466`) on a tokio worker while holding the engine read guard taken at `:5446`.

**Impact.** A ~50-byte request kills the process by OOM or pins a worker forever holding a `std::sync::RwLockReadGuard` on the engine — which, being write-preferring, then blocks writers and subsequently all readers. `#[tokio::main]` at `:16185` has no `worker_threads` override and `fly.toml` sets `cpus = 4`, so four concurrent `burn_in` requests exhaust every worker; only three `spawn_blocking` calls exist in the whole binary (`:10135`, `:12380`, `:16654`), none on this path.

**Evidence.**
```rust
// src/geometry/generative_flow.rs:338
        let mut path = Vec::with_capacity(config.n_steps + 1);

// src/geometry/generative_flow.rs:377-385 — validate() never inspects the loop bounds
        if config.dt <= 0.0 { return Err(GenerativeFlowError::NonPositiveStep(config.dt)); }
        if config.temperature < 0.0 { return Err(GenerativeFlowError::NegativeTemperature(config.temperature)); }
        Ok(())
```

**Fix sketch.** Add explicit caps in `validate` — it already exists and every entry point calls it — rejecting `n_steps`, `burn_in`, and `n_samples` above typed ceilings. Cap the response size too: `n_steps * dim * 8` is the trajectory the server must serialize. Use `checked_add`/`checked_mul` rather than `n_steps + 1` and `n_samples * stride` (`:283`). Move the solve to `spawn_blocking` and drop the engine read guard before it.

---

### H10 — Raw AES-256 gauge keys written verbatim into the WAL beside their ciphertext

`src/wal.rs:1447` — **precondition: requires filesystem, backup, snapshot, or object-store read access. Not reachable over HTTP.**

> **Contested severity.** The crypto pass rated this HIGH on the rubric's "key/secret exposure" clause, noting the object-store sync widens the blast radius past the host. The data-boundary pass rated it LOW because no outside caller can trace to it and the threat model presumes prior compromise. Recorded at HIGH with the precondition stated in the header, on the grounds that a diligence reader will weigh the artifact, not the trigger — but the honest reading is that this is *not* remotely exploitable, and the fix order below reflects that.

**Attack path.** Any bundle with an `opaque`/`indexed`/`probabilistic` fiber field gets a `GaugeKey` installed at CREATE BUNDLE (`src/bin/gigi_stream.rs:12817`). Every schema write goes through `encode_schema` (`src/wal.rs:1412`), which walks `schema.gauge_key` and pushes a variant tag followed by the **raw 32-byte key**: `Opaque` (`:1447-1450`), `Indexed` (`:1453`), `Probabilistic.bucket_key` (`:1466`), plus the Affine `scale`/`offset` f64s (`:1444-1445`). `log_create_bundle` (`:438`) wraps that into a WAL entry and `Engine::create_bundle` (`src/engine.rs:1317`) calls it as its first statement, so the key hits `gigi.wal` before the bundle exists in memory. `decode_schema` (`:1578-1600`) reads it straight back, confirming this is the live format. The record inserts that follow in that same file are the ciphertext produced by that key. `tigris_push` (`src/bin/gigi_stream.rs:15548`) then runs `aws s3 sync <data_dir>/ s3://<bucket>/` with `--exclude "*.tmp"` as the only filter, spawned on both the fast path (`:16685`) and the slow path (`:16838`), so WAL and snapshots leave the machine together to the bucket named in `fly.toml` (`TIGRIS_BUCKET_NAME = "gigi-snapshots"`).

**Impact.** Encryption at rest provides no confidentiality against anyone holding the data volume, a backup, or the Tigris object: key and ciphertext arrive in one artifact. The `WITH ENCRYPTION SEED FROM ENV` ceremony is defeated regardless of how carefully the seed is supplied, because the derived per-field keys are persisted in the clear anyway. Compounds with H11: one recovered field key yields every field key under the same seed. Recovery requires re-keying every affected bundle, not merely rotating the WAL.

The HTTP surface was actively checked and is clean: `get_schema` (`:10682-10730`), `export_bundle` (`:10876`), and `export_dhoom` (`:10905-10930`) all hand-build JSON from field names/types/weights and never touch `gauge_key`, and `BundleSchema` carries no `Serialize` derive (`src/types.rs:410` derives only `Debug, Clone`).

**Evidence.**
```rust
// src/wal.rs:1447-1455
                crate::crypto::FieldTransform::Opaque { key } => {
                    buf.push(0x02);
                    buf.extend_from_slice(key);
                }
                crate::crypto::FieldTransform::Indexed { key } => {
                    buf.push(0x03);
                    buf.extend_from_slice(key);
                }
```

**Fix sketch.** Do not persist derived key material. Persist the non-secret schema shape plus the seed *source* (`EncryptionSeedSource`) and re-run `GaugeKey::derive(&seed, &fiber_fields)` at load time — the derivation is deterministic, which is the entire point of `seed_env`. Note this covers only the `Env`/`Hex` sources: `EncryptionSeedSource::Random` (`src/bin/gigi_stream.rs:12781`) generates a seed stored nowhere else, so a KEK-wrapping path is **required**, not optional — wrap derived material under a key that lives only in the environment and keep the wrapped blob out of the stream that syncs to Tigris. Independently, push the WAL to a different bucket from the snapshots so one object leak is not both halves. Add a regression test asserting no 32-byte key value appears in `encode_schema` output.

---

### H11 — Per-field KDF is a non-cryptographic, fully invertible 64-bit mixer, not HKDF-SHA256

`src/crypto.rs:855` (mixer at `:876`)

**Attack path.** `derive_field_key` builds the 32-byte AES-256 key for Opaque/Indexed/Probabilistic modes as four `mix_hash` outputs over `seed || purpose || field_name` with four fixed IVs. Every step of `mix_hash` is a bijection: the multipliers `0x2d358dccaa6c78a5`, `0xff51afd7ed558ccd`, `0xc4ceb9fe1a85ec53` are all odd (invertible mod 2^64), and `h ^= h >> 33` is self-inverse because 33·2 ≥ 64. `purpose` is a constant and `field_name` is public schema metadata.

**Verified by runnable proof-of-concept** (scripts in the session scratchpad, reproducing against a fresh `os.urandom` seed each run). Given **any one** 32-byte field key: un-finalize the four lanes, peel the known `purpose||field_name` suffix backwards to recover the four post-seed states, then re-run forward with a different `purpose'||field_name'`. Output: `:opaque: salary True`, `:indexed: email True`, `:prob_bucket: dob True`, `:opaque: ssn True` — exact key match every time, **without ever learning the seed**.

**Impact.** Total loss of key separation. Domain separation via `purpose` (`:855-866`) is cosmetic. The blast radius of any single field-key disclosure is every encrypted field in the deployment — and H10 puts those keys in the WAL and the Tigris bucket, so the two compound directly. This is also a **spec divergence**: `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md:172` specifies `field_seed = HKDF-SHA256(seed, salt = f.name || f.field_type)`, the `hkdf` crate is already a dependency (`Cargo.toml:160`), and `src/integrity.rs:101` uses it correctly with a proper salt. The implementation simply diverged from its own specification — which is the form of finding an external cryptographer will locate fastest, and the one hardest to explain after the fact.

A second consequence was investigated and is being reported *tempered*: `derive_affine` (`:286`) reuses `mix_hash` over `seed||":affine:"||field_name` with the **same first two IVs** `derive_field_key` uses (`:863-864`), and the PoC confirms the post-seed states are bit-identical (`affine shares S1/S2 with opaque key: True`), so affine parameters and Opaque keys are provably not independent. However, affine ciphertext is never returned over the API — every read path decrypts (`src/bundle.rs:2145`, `:2742`, `:2897`) — so recovering `scale`/`offset` from known plaintexts requires raw storage access, and anyone with that already holds the keys outright via H10. The finding stands on the inversion result, which requires no known plaintext at all.

**Evidence.**
```rust
// src/crypto.rs:863-866
    let h1 = mix_hash(&input, 0x517cc1b727220a95);
    let h2 = mix_hash(&input, 0x6c62272e07bb0142);
    let h3 = mix_hash(&input, 0xff51afd7ed558ccd);
    let h4 = mix_hash(&input, 0xc4ceb9fe1a85ec53);

// src/crypto.rs:876-882 — every step invertible
fn mix_hash(data: &[u8], seed: u64) -> u64 {
    let mut h = seed;
    for &b in data {
        h = h.wrapping_mul(0x2d358dccaa6c78a5).wrapping_add(b as u64);
        h ^= h >> 33;
    }
```

**Fix sketch.** Replace the entropy source of `derive_field_key`, `derive_affine`, `derive_probabilistic`, and `derive_orthogonal_matrix` with the HKDF the spec already calls for and that `src/integrity.rs:100-106` already models: `Hkdf::<Sha256>::new(Some(b"gigi-gauge-v1"), seed)` then `hk.expand(&[purpose, field_name.as_bytes()].concat(), &mut okm)`. Use disjoint `info` strings per mode so affine parameters and opaque keys come from independent expand outputs rather than shared IVs. This is a wire-format break for existing encrypted bundles: version the gauge-key marker byte in `encode_schema` and re-key on upgrade. Keep `mix_hash` for the non-security hashing it is fine for, and add a test asserting no security key is produced by it.

---

## MEDIUM

---

### M1 — `WITH ENCRYPTION SEED FROM ENV` is an arbitrary environment-variable oracle

`src/bin/gigi_stream.rs:12796` (seed parse at `src/crypto.rs:890`), duplicated on ROTATE_KEY at `:12873-12891`

**Attack path.** An authenticated non-owner tenant POSTs to `/v1/gql`: `CREATE BUNDLE probe (id INT BASE, x TEXT FIBER ENCRYPTED) WITH ENCRYPTION SEED FROM ENV GIGI_JWT_SECRET;`. The grammar accepts any bare identifier as the variable name with no prefix restriction or allowlist (`src/parser.rs:2708-2717`, tokenizer at `:2168-2170`), and the handler calls `std::env::var(name)` on it at `:12796` — before `engine.create_bundle`, so the tenant needs no permission to create anything. Three distinguishable 400s come back:

1. unset → `env var <NAME> not set` (`:12811`) — existence oracle for any variable in the container
2. set, wrong length → `Encryption seed must be 64 hex characters (32 bytes), got {N}` (`src/crypto.rs:892-897`) — the **exact trimmed byte length** of the live secret
3. set, 64 chars, non-hex → `Invalid hex at position {i*2}` (`:901`) — index of the first non-hex byte

Reachable because the `/v1/gql` route is ungated (H4) and the CreateBundle arm consults no claims; the encrypted-field trigger at `:12774-12778` is caller-controlled.

**Verified live** with an `owner:false` token: `GIGI_JWT_SECRET` → "got 34"; `GIGI_API_KEY` → "got 10"; `PATH` → "got 1793"; `NO_SUCH_VARIABLE_XYZ` → "not set".

**Impact.** An untrusted tenant enumerates which secrets the process holds — `GIGI_API_KEY`, `GIGI_JWT_SECRET`, `JG_KV_ENCRYPTION_SEED`, `AWS_SECRET_ACCESS_KEY`, `TIGRIS_BUCKET_NAME` — and learns each one's exact length plus the position of its first non-hex character. The value is deployment reconnaissance and spotting a short, brute-forceable key. **Two claims from the original report are withdrawn:** no content is ever echoed (the messages carry a length and an offset, never the value), and "length materially narrows an offline attack on `GIGI_JWT_SECRET`" overstates it — for a high-entropy secret, knowing it is 34 rather than 64 characters does not meaningfully reduce the search space. This is not reachable from `/v1/public/gql`: `validate_public_stmt` excludes CreateBundle and RotateKey. It survives a fix to H4, because a tenant may legitimately CREATE BUNDLE inside their own namespace.

**Evidence.**
```rust
// src/crypto.rs:890-897
pub fn seed_from_hex(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(format!(
            "Encryption seed must be 64 hex characters (32 bytes), got {}",
            hex.len()
        ));

// src/bin/gigi_stream.rs:12803 — echoed verbatim to the caller
  Json(serde_json::json!({"error": format!("invalid encryption seed in env {name}: {e}")})),
```

**Fix sketch.** Restrict `FROM ENV` to a prefix allowlist (e.g. names matching `GIGI_SEED_*`) or an operator-supplied list resolved at boot, and reject anything else before calling `std::env::var`. Collapse all three failure modes into one opaque message — `seed source unavailable or malformed` — and log the detail server-side keyed by request id. Apply to both the CREATE_BUNDLE (`:12795-12815`) and ROTATE_KEY (`:12873-12891`) arms. Require owner claims for the verb.

---

### M2 — `/v1/admin/*` is authenticated but not authorized

`src/bin/gigi_stream.rs:12379` (routes `:16271-16274`)

**Attack path.** A non-owner tenant authenticates normally. `parse_bundle_segment("/v1/admin/snapshot")` returns `None` (`:1554`), so namespace enforcement never fires. `admin_snapshot` takes `State` only, never reads `GigiClaims`, and immediately does `state.engine_write()` + `engine.snapshot_with_report()` on a blocking task — a full DHOOM write of every bundle plus WAL compaction, holding the process-wide write lock throughout. `snapshot_with_report` bounds each bundle by `compaction_policy.per_bundle_timeout_secs`, default `Some(600)` (`src/engine.rs:211`, `:2221-2225`), but nothing bounds the sum and nothing serializes concurrent invocations.

**Verified live** with the `owner:false` token: `POST /v1/admin/snapshot` returned `{"status":"ok","total_records_snapshotted":6,...}`; `POST /v1/admin/log-level {"level":"TRACE"}` returned `{"status":"ok","level":"TRACE"}`.

**Impact.** Two outcomes. (1) Any tenant stalls the whole single-node service on demand — every request blocks on the write lock, plus disk and Tigris egress pressure — and `/v1/health`'s `try_read` Err arm returns 200 `"ok"` (`:1642`, see L9) so the readiness probe stays green while the lock is held. (2) Anti-forensics: `update_log_config` (`:12441-12480`) is reachable the same way and sets `cat_query`/`cat_bundle`/`cat_system` straight from the body, while `Logger::update_config` (`src/observability.rs:435-440`) force-restores only `cat_audit` — and reads are not audit events. A tenant silences query logging, then exercises H4, and nothing lands in `_gigi_query_log`. Neither action is attributable to a namespace, and neither requires the owner flag the token explicitly denies. A non-owner triggering WAL compaction (`src/engine.rs:2491`) is also a durability-affecting operation over other tenants' data.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:12379-12384
async fn admin_snapshot(State(state): State<Arc<StreamState>>) -> impl IntoResponse {
    let snapshot = tokio::task::spawn_blocking(move || {
        let mut engine = state.engine_write();
        engine.snapshot_with_report()
    })
    .await;
```

**Fix sketch.** Put every `/v1/admin/*` route on a dedicated sub-router with a `route_layer` that reads `GigiClaims` and 403s unless `claims.owner` — as a layer, so a newly added admin route inherits the guard rather than needing to remember it. Independently, debounce `admin_snapshot`: reject if a snapshot ran within the last N seconds, and allow only one concurrent invocation regardless of caller.

---

### M3 — `decrypt_value` panics on AEAD failure; `drop-field` makes an encrypted bundle permanently unreadable

`src/crypto.rs:169` (trigger at `src/bundle.rs:3975`, route `src/bin/gigi_stream.rs:10778`)

**Attack path.** `FieldTransform::decrypt_value` calls `aead_decrypt(...).expect("AEAD decrypt failed — ciphertext tampered or wrong key/AAD")`. `aead_decrypt` (`:694`) correctly returns a uniform `Err(())` for every failure mode, so the primitive leaks nothing — but the caller converts that single error into an unconditional panic on the query hot path (reached from `src/bundle.rs:1593`, `:1669`, `:2144`, `:2896`).

The reachable trigger, which the original pass reported as absent: `BundleStore::drop_field` (`src/bundle.rs:3975-4022`) removes the fiber field at position `pos` from `schema.fiber_fields` and splices the value out of every fiber vector, **but never touches `schema.gauge_key.transforms`**, which keeps its original length and order. `decrypt_fiber` indexes transforms positionally and rebuilds the AAD from the post-drop index and name (`src/crypto.rs:427-460`, `build_aad` at `:576`). So for every `i >= pos`, ciphertext encrypted under `transform[i+1]` with AAD `bundle|i+1|oldname` is decrypted with `transform[i]` and AAD `bundle|i|newname`. Wrong key **and** wrong AAD.

Full path: `POST /v1/gql` `CREATE BUNDLE t (id INT BASE, a TEXT FIBER, b TEXT FIBER) ENCRYPTED;` (TEXT defaults to Opaque via `EncryptionMode::default_for_type`, `src/parser.rs:2993`) → insert a record → `POST /v1/bundles/t/drop-field {"field":"a"}` (handler `:10778`, straight through `BundleMut::drop_field` at `src/mmap_bundle.rs:2219` with no encryption check) → any subsequent read panics. Dropping a non-last Opaque field is required; dropping the last one is harmless.

**Impact.** One authenticated HTTP call turns a recoverable per-record decrypt error into permanent unreadability of that bundle: every subsequent read aborts the connection task, and the remaining encrypted columns are unrecoverable through the normal path. Blast radius is bounded — `Cargo.toml` sets no `panic = "abort"` (there is no `[profile]` section at all), there is no `CatchPanicLayer`, and `engine_read`/`engine_write` are poison-proof (`src/bin/gigi_stream.rs:319-330`), so the panic kills the connection rather than the process. `drop-field` **is** namespace-gated, so a non-owner can only do this to their own bundles; for a shared-API-key deployment it is self-inflicted. That bounding is what holds it at MEDIUM rather than HIGH.

**Evidence.**
```rust
// src/crypto.rs:167-172
FieldTransform::Opaque { key } => match w {
    Value::Binary(bytes) => {
        let plaintext_bytes = aead_decrypt(key, bytes, aad)
            .expect("AEAD decrypt failed — ciphertext tampered or wrong key/AAD");
        bytes_to_value(&plaintext_bytes)
    }
```

**Fix sketch.** Two independent changes, both needed. (a) Make `drop_field` reject on an encrypted bundle, or splice the corresponding `gauge_key.transforms` entry in the same operation and re-encrypt the affected columns under the new indices — silently reindexing one side of an AAD binding is the actual defect. (b) Change `decrypt_value` to return `Result<Value, DecryptError>`, propagate through `decrypt_fiber`, and let the HTTP layer map it to a 500 with a generic body; keep the single opaque error variant `aead_decrypt` already provides so wrong-key and corrupt-data stay indistinguishable. `aead_encrypt` at `:685` has the same `.expect` shape and should move with it.

---

### M4 — Rate limiting is disabled in production, in front of the anonymous endpoint

`src/bin/gigi_stream.rs:356` (middleware `:1580`)

**Attack path.** `state.rate_limit` reads `GIGI_RATE_LIMIT` with `.unwrap_or(0u32)`, and `rate_limit_middleware` short-circuits on its first line when zero. `fly.toml`'s `[env]` block sets `PORT`, `GIGI_DATA_DIR`, `GIGI_INGEST_DIR`, `TIGRIS_BUCKET_NAME`, `GIGI_APP_BUNDLES`, `GIGI_PUBLIC_BUNDLES` — and nothing else. So there is no per-IP limit on any route, authenticated or anonymous.

**Impact.** This is the multiplier on H1, H2, H3, and every exhaustion finding below: it is what makes them repeatable rather than one-shot. It also enables direct anonymous exhaustion in its own right — `COVER chembl ALL;` in a loop against `exec_result_to_response` (`:15391`), which materializes the whole result as `Vec<serde_json::Value>` and then serializes the entire body in memory, capped only by `GIGI_QUERY_MAX_ROWS` default 10 000 000 (`src/bundle.rs:2424`).

Two latent problems apply the moment it is switched on: (a) `GIGI_TRUST_PROXY` is also unset, so the middleware keys on `ConnectInfo` (`:1596`), which behind Fly's edge is the proxy address — every client on earth shares one bucket; (b) with trust-proxy on, the map key becomes the attacker-supplied `x-forwarded-for` value (`:1587-1593`), and `tracker.entry(ip).or_default()` (`:1607`) inserts a key that `entries.retain(...)` (`:1610`) never removes, with no sweep task — a client rotating XFF values grows the map without bound.

**Caveat, stated plainly:** environment variables can also arrive via `flyctl secrets`, which is not visible in-repo. "Off in production" is a strong inference from the committed config, not a direct observation. **Run `flyctl config env` to confirm before acting.**

**Evidence.**
```rust
// src/bin/gigi_stream.rs:356-359
        let rate_limit = std::env::var("GIGI_RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0u32); // 0 = unlimited
```

**Fix sketch.** Set `GIGI_RATE_LIMIT` in `fly.toml` and default it to a nonzero value in code; give `/v1/public/gql` its own much tighter budget. Set `GIGI_TRUST_PROXY` at the same time or the limit is meaningless behind Fly. Replace the unbounded `HashMap<String, Vec<Instant>>` with a bounded LRU or sharded token bucket with a periodic sweep, drop entries whose `Vec` is empty after `retain`, validate the XFF-derived value is a well-formed address before using it as a key, and trust only the rightmost hop Fly appends.

---

### M5 — KDE max-density cache is keyed on a caller-supplied f64; O(N²·D) per request

`src/vector_cache.rs:276`

**Attack path.** `POST /v1/bundles/{b}/brain/confidence` (or `/intent_gate`, or `/confidence_with_explain`) with `{"fields":[…],"query":[…],"bandwidth":<fresh float each time>}`. The handler passes `req.bandwidth` through unchanged when positive (`src/bin/gigi_stream.rs:7037-7044`) into `kde_normalized_cached` (`:7051`), which calls `max_density_cached`. The cache key is `bandwidth.to_bits()` — an exact bit comparison (`src/vector_cache.rs:277`), so perturbing the mantissa by one ULP is a guaranteed miss. The miss path takes a **write** lock (`:289`) and holds it across `max_density_of_matrix` (`:296`), the explicit N×N×D triple loop at `:254-267`. The codebase's own comment puts that at ~4 s for N=10 000, D=384 (`:244-245`); `src/bin/gigi_stream.rs:6899` documents 35 s pre-cache. This cache exists specifically to amortize that cost, and its key is attacker-controlled.

**Impact.** A ~200-byte request buys seconds of pinned CPU on a 4-worker runtime, sustained indefinitely, with the engine read guard held. Concurrent requests also serialize behind the `max_density_by_bw` write lock, so throughput collapses for well-behaved callers on the same bundle. This survives the overlay caveat that narrowed L5, because the handler goes through `heap_or_promote` (`:7024`) rather than `as_heap()`. It requires the tenant to have first uploaded a wide bundle, which is what holds it at MEDIUM. The unbounded `HashMap` half is real but negligible — 16 bytes per entry, each costing the attacker seconds of compute to create.

**Evidence.**
```rust
// src/vector_cache.rs:276-297 (abridged)
pub fn max_density_cached(cached: &CachedMatrix, bandwidth: f64) -> f64 {
    let bw_bits = bandwidth.to_bits();
    { ... if let Some(&v) = map.get(&bw_bits) { return v; } }
    let mut map = match cached.max_density_by_bw.write() { ... };
    let v = max_density_of_matrix(&cached.matrix, bandwidth);   // under the write lock
    map.insert(bw_bits, v);
```

**Fix sketch.** Quantize the key — round bandwidth to a fixed number of significant digits, or snap to a small set of allowed multiples of the fit-derived σ — so nearby bandwidths share an entry. Bound the map with a small capacity plus eviction. Compute `max_density_of_matrix` **outside** the write lock and take the lock only to insert. Add an N ceiling above which normalized confidence is refused rather than computed.

---

### M6 — Public SELECT/COVER have no LIMIT and triple-materialize the bundle

`src/bin/gigi_stream.rs:15114`

**Attack path.** Anonymous `POST /v1/public/gql` `{"query":"SELECT * FROM chembl"}`, ~40 bytes. `validate_public_stmt` allows `S::Select` for any allowlisted bundle (`:12568`). `Statement::Select` has **no limit field in the AST** (`src/parser.rs:368-373`), and the executor opens with `let all_rows: Vec<_> = store.records().collect();`. `BundleStore::records()` reconstructs an owned `Record` (a `HashMap<String, Value>`) per row (`src/bundle.rs:2853-2858`) — a full deep materialization, not a borrow. `exec_result_to_response` then builds a second full copy as `Vec<serde_json::Value>` (`:15391-15393`) and a third as the serialized body. `COVER <bundle>` with no `FIRST` is the same (`:13908-13910`). On the mmap path this is worse, not better: `OverlayBundle::records` (`src/mmap_bundle.rs:1058-1063`) first collects every overlay record into a `Vec<Record>` plus a `HashSet<String>` of keys.

**Impact.** Peak transient is roughly 3× the logical bundle size with per-record `HashMap` overhead on top, plus an unbounded response body, from a 40-byte unauthenticated request with no rate limit in front of it. On today's allowlisted data (chembl ~2 200 rows, stations 480, tetmesh_demo ~5 760 rows carrying vector fibers) that is tens of MB transient and a multi-MB body per request — a real amplification, though the OOM framing is conditional on a larger bundle being added to the allowlist rather than on current state.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:15112-15114
        Statement::Select { columns, condition, .. } => {
            use gigi::parser::SelectCol;
            let all_rows: Vec<_> = store.records().collect();
```

**Fix sketch.** Add `limit: Option<usize>` to `Statement::Select` and apply a server-side default cap unconditionally on the public path plus a maximum on the authenticated path. Stream rather than collect — the executor could return an iterator and the response layer write NDJSON, as `/query-stream` already does. Enforce a maximum row count in `validate_public_stmt` for Select/Cover regardless of what the query asks for.

---

### M7 — `GET /v1/bundles/{name}/points` defaults `limit` to `usize::MAX`

`src/bin/gigi_stream.rs:10260` (route `:16235`)

**Attack path.** An authenticated tenant issues the GET with no query string. `limit` is `None`, so `take_count = limit.unwrap_or(usize::MAX)`, and the chain `store.records().skip(0).take(usize::MAX).map(record_to_json).collect()` (`:10262-10267`) reconstructs and JSON-converts every record into a single `Vec<serde_json::Value>`, serialized as one body — while holding the engine read guard taken at `:10245`. The comment directly above reads *"Streaming pagination — never buffer the entire bundle"*; the `.collect()` on `:10267` is exactly that buffering.

**Impact.** One credentialed GET with no parameters allocates the whole bundle twice plus the serialized body; on an overlay-backed bundle `records()` adds another full materialization (`src/mmap_bundle.rs:1058-1063`). Held at MEDIUM rather than HIGH because the namespace middleware confines a non-owner to bundles they populated themselves, so this is a per-request 2–3× transient and an unbounded response rather than foreign-data access — but the transient lands in a shared 4-worker process, so concurrency multiplies it across tenants.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:10258-10267
    // Streaming pagination — never buffer the entire bundle
    let start = offset.unwrap_or(0);
    let take_count = limit.unwrap_or(usize::MAX);

    let json_records: Vec<serde_json::Value> = store
        .records()
        .skip(start)
        .take(take_count)
        .map(|r| record_to_json(&r))
        .collect();
```

**Fix sketch.** Default `limit` to a sane page size (100–1000) and clamp any caller-supplied value to a maximum. If unbounded listing is genuinely needed, make it a separate streaming route writing NDJSON incrementally. Fix the comment either way — a comment that describes the opposite of the code is how this survived review.

---

### M8 — `/v1/bundles/{name}/stream` bypasses the body limit up to 256 MB and triple-materializes it

`src/bin/gigi_stream.rs:2293`

**Attack path.** The handler takes `body: axum::body::Body` raw (`:2275`), so axum's 2 MB `DefaultBodyLimit` on the `Json` extractor does not apply; `to_bytes(body, 256 * 1024 * 1024)` (`:2293`) is the only ceiling. `String::from_utf8_lossy(&bytes)` (`:2302`) borrows for valid UTF-8 and allocates a full second copy for invalid — so deliberately malformed bytes double the peak. Lines `:2308-2325` then expand each line into a `Record` (`HashMap<String, Value>` with owned String keys) into an unbounded `Vec`. Only at `:2328` is the write lock taken.

**Impact.** 256 MB of compact NDJSON expands several-fold through per-field String keys and HashMap overhead, easily into the low GB per request. There is no admission control to blunt concurrency — `tower-http` is cors-only (`Cargo.toml:143`) and `tower` is a dev-dependency (`:362`), so `ConcurrencyLimitLayer` and `TimeoutLayer` are not compiled in. Four concurrent uploads put multiple GB of transient on the heap simultaneously. Also a WAL-growth vector: the parsed batch is WAL-logged, so 256 MB in becomes a proportional durable write. Requires sustained volume, and the bundle-exists check at `:2280-2290` scopes it to the tenant's own namespace — hence MEDIUM.

**Evidence.**
```rust
// src/bin/gigi_stream.rs:2272-2302 (abridged)
async fn stream_ingest(
    State(state): State<Arc<StreamState>>,
    Path(name): Path<String>,
    body: axum::body::Body,
) -> ... {
    // Read body (cap at 256MB to prevent abuse)
    let bytes = to_bytes(body, 256 * 1024 * 1024).await.map_err(...)?;
    let text = String::from_utf8_lossy(&bytes);
```

**Fix sketch.** Lower the per-request ceiling substantially (a few MB) and process the body as a stream — read and insert in fixed-size chunks rather than materializing `bytes`, `text`, and `records` in full. Add `tower::limit::ConcurrencyLimitLayer` on the ingest routes and a global `TimeoutLayer` (see L14) so a slow or huge upload cannot occupy a worker slot indefinitely.

---

## LOW

---

### L1 — API key accepted from the URL query string on every route, not just WS upgrades

`src/bin/gigi_stream.rs:1393`

The doc comment at `:1311` and the 2026-06-04 hardening note at `:1322-1326` both describe `?api_key=` as a WebSocket-upgrade-only transition path. The code applies no upgrade check: the `query_key` closure at `:1393-1402` runs for every request regardless of method or path, and its result is accepted at `:1403-1407`. The `?gigi_token=` bearer path at `:1431-1440` has the same shape. **Verified live on a plain HTTP GET:** `GET /v1/bundles?api_key=testkey123` returned the full bundle list; the same GET without the parameter returned 401.

*Impact.* An owner-equivalent credential lands in Fly edge access logs, browser history, Referer headers, and JS error-reporter URL captures — surfaces the operator does not control. Not exploitable on its own; it needs a caller to actually use the query form. What makes it worth fixing is that the mitigation reads as **done** in review and is not, so it will not be caught again.

*Fix.* Gate both query-string branches on the request being a genuine WebSocket upgrade (`connection: upgrade` + `upgrade: websocket`, or `path.starts_with("/ws")`). The subprotocol path at `:1332-1352` already covers real WS clients, so the query form can be dropped outright once callers have moved. Correct the two comments.

---

### L2 — Auth fails open to owner claims when both credential env vars are absent

`src/bin/gigi_stream.rs:1463`

`auth_middleware` ends with: if no credential produced claims and both `state.api_key` and `state.jwt_secret` are `None`, assign `GigiClaims::owner_via_api_key()` — `owner:true`, `exp:u64::MAX` — to the anonymous request. Both fields come from `std::env::var(...).ok()` at `:333-334`. **Verified live** with both unset: an unauthenticated `POST /v1/gql` `CREATE BUNDLE anon_owned (...)` returned `{"status":"ok"}` and `SHOW BUNDLES` listed it. `GET /v1/health` returned `{"status":"ok"}` with no auth field, and grepping the captured stdout/stderr for `auth|API|WARN|key` produced **zero lines** — the startup banner at `:16810` silently omits the `-H 'X-Api-Key'` hint rather than warning.

*Impact.* A single missing environment variable — a secret unset on a new app, a machine started outside the normal deploy, a typo in the secret name, a restore into a fresh environment — silently converts a production database into an open one, with no log line, no metric, and no health-check signal. Requires operator error to trigger, so LOW; the failure mode is total and silent.

*Fix.* Make open mode opt-in and loud: require `GIGI_ALLOW_ANONYMOUS=1` for the fail-open branch, `exit(1)` at startup when neither credential source is configured and the flag is absent, and surface `"auth":"disabled"` in `/v1/health` plus a repeated WARN line.

---

### L3 — Unredacted GQL text — encryption seeds and encrypted-field plaintext — logged to stdout

`src/observability.rs:545` — *downgraded from MEDIUM during verification*

`src/bin/gigi_stream.rs:12707` calls `state.logger.query_start(&req_id, "gql", query, ...)` with the raw request body **before parsing**; `query_start` attaches it as `.field("raw_gql", raw_gql)` with no redaction (a grep for `redact|truncate_query|statement_preview` in `observability.rs` returns nothing). `emit` gates only on category (`:413-420`; `cat_query` defaults true at `:184`), and `LogIngester::run` (`:955-963`) `println!`s the serialized event whenever `stdout_enabled`, which `LogConfig::default()` sets to `true` (`:178`). `query_error` (`:12715`) and `query_complete` (`:12730`) carry `raw_gql` too. Two concrete secrets travel this path: `CREATE BUNDLE b WITH ENCRYPTION SEED '<64 hex>'` — the master seed — and `INSERT INTO jg_kv (key,payload) VALUES (…)`, the exact field `fly.toml` marks `encrypted:"opaque"`.

*Impact.* The at-rest encryption boundary is bypassed at the log layer: plaintext that is AEAD-encrypted in the store is emitted in cleartext to a lower-trust sink, and the seed that decrypts it is emitted alongside. **Downgraded** because there is no outside-caller read path — `log_bundle_writer` (`:15938-15960`) copies only `statement_type`, `bundle`, `request_id`, `slow`, and `error_msg` into `_gigi_query_log`, deliberately excluding `raw_gql`, so the seed never lands in a queryable bundle. Recovery requires Fly log or drain access, i.e. a second system. Secrets hygiene, not an externally-extractable key exposure.

*Fix.* Redact before the event is built: strip string literals from `raw_gql` (keep verb, bundle, and a literal count), and hard-drop any statement matching `WITH ENCRYPTION SEED` / `ROTATE KEY` regardless of setting. Log the statement *shape*, not the statement. If full text is wanted for debugging, put it behind a non-default `GIGI_LOG_RAW_GQL` flag documented as disabling the encryption boundary.

---

### L4 — `_gigi_query_log` is tenant-readable and `error_msg` echoes callers' statement literals

`src/bin/gigi_stream.rs:15574`

`init_system_bundles` declares `_gigi_query_log` with an `error_msg` text field. On a parse failure `:12715` calls `query_error(..., "ParseError", &e.to_string(), 400)` and `log_bundle_writer` (`:15938-15945`) copies `error_msg` verbatim into the bundle. Because `/v1/gql` has no namespace check (H4) and no `_gigi_` read guard exists in the executor, any tenant runs `COVER _gigi_query_log ALL;` and reads every other caller's request ids, targeted bundle names, statement types, timings, and those echoed messages.

*Two citations from the original report are corrected.* `src/parser.rs:2223` and `:2233` no longer use `{other:?}` — they route through `token_or_end` → `Token::human` (`:1909-1929`), which renders a `Str` as `string '<first 24 chars>…'`, truncated. `:2735` also cannot echo a literal. The class is nonetheless real at ~77 other sites still formatting `{other:?}` on `Option<Token>` with no truncation; `parse_usize` at `src/parser.rs:6925-6929` is the cleanest instance (`COVER b FIRST 'literal'` reproduces the literal in full), plus `:2795` and `:2827`.

*Impact.* Cross-tenant leak of operational metadata (who queried which bundle, when, how long) and, for statements that fail to parse *after* a literal, of caller-supplied values. LOW: the metadata leak is certain, the secret-in-literal leak requires another tenant to have made a specific parse mistake, and the cross-tenant readability is entirely inherited from H4.

*Fix.* (a) Treat `_gigi_*` as owner-only on **read** in the GQL path, not merely write-protected on two REST handlers. (b) Sanitize parse errors before they become `error_msg` — report position and expected-token class, never the literal's contents. Audit the ~77 `{other:?}` sites and route them through the existing truncating `Token::human`.

---

### L5 — Public SPECTRAL builds an uncapped O(Σ|group|²) graph; the 4096 guard runs after the build

`src/spectral.rs:1883` — *downgraded from MEDIUM during verification*

`validate_public_stmt` allows `S::Spectral { bundle, .. }` for any allowlisted bundle (`src/bin/gigi_stream.rs:12575`) — the `..` includes `FULL`. `field_index_graph` (`src/spectral.rs:280-303`) has no vertex cap, record cap, group cap, or time budget: for every distinct value of every indexed field it does a nested loop over the whole group inserting into a `HashSet<BasePoint>`, i.e. Σ_v |group(v)|². The 4096-vertex "dense eigensolver threshold" check at `:1894` runs **after** the quadratic build at `:1883`, so it cannot function as a cost control.

*Downgraded* because the impact is not reachable in the deployed steady state. Both SPECTRAL arms require a heap store — FULL does `store.as_heap().ok_or_else(...)` and the scalar arm is `store.as_heap().map(spectral_gap).unwrap_or(0.0)` (`src/bin/gigi_stream.rs:14069-14091`). After boot, `Engine::open_mmap` puts every snapshotted bundle into `mmap_bundles` as `BundleRef::Overlay` (`src/engine.rs:702-708`, `:1752-1759`), and inserts into an overlay never promote it to heap (`:1336-1345`) — so on any machine that has restarted since its last snapshot, SPECTRAL returns 0.0 or a 400 having done zero work. The window is between a manual re-seed and the next restart. The magnitudes were also overstated: `chembl` is created with no `indexed` list at all (`examples/seed_demo_bundles.py`), so the graph is empty and FULL exits at `:1887`; `stations` is 480 rows over 7 bands. Only `tetmesh_demo` has real cost — ~5 760 records indexed on a 4-value `level`, ≈1.25e7 edges, a few hundred MB and seconds.

*Note the cheaper arm is worse.* The non-FULL scalar path builds the same graph at `:329` and then runs 300 power iterations doing a HashMap lookup per edge (`:367-379`, `:383-396`) ≈ 3.7e9 lookups — more pinned CPU than the FULL eigensolve, and no `FULL` keyword required.

*Fix.* Move a record-count / vertex-count precheck ahead of `field_index_graph` (`store.len()` is O(1) and already the right proxy). Cap group size inside the loop, since the memory is the group-size square rather than the vertex count. Drop SPECTRAL from the public allowlist, or gate the public arm to the λ₁-only scalar on bundles under an explicit ceiling. Run the solve under `spawn_blocking` with a deadline so it cannot pin a worker or hold the engine read lock.

---

### L6 — No finite-value guard on writes; a `1e400` insert temporarily disables variance-gated analytics

`src/parser.rs:2274` — *downgraded from MEDIUM during verification*

The tokenizer parses numeric literals with `str::parse::<f64>()` (`src/parser.rs:2125`, `:2165`), which saturates to `+inf` without error, and `parse_literal` passes it through with no `is_finite` check. A grep for `is_finite|is_nan` across `src/types.rs`, `src/engine.rs`, `src/bundle.rs` shows no guard on any write path. `SECTION inf (id: 3, a: 1e400, b: 5);` returns `{"status":"ok"}`, reads back as JSON `null` (`src/parser.rs:9950` gates emission on `is_finite`), and afterwards `POST /v1/bundles/inf/reduce` returns a permanent 422 because the NaN sd fails `sd > f64::EPSILON` at `src/ml/reduce.rs:73`.

*Two impact claims did not survive testing.* (a) "All four cluster methods returned 422" did not reproduce — with one poisoned and one clean fiber, kmeans returned a full 200; `src/ml/cluster.rs:349` requires `dim < 1`, so cluster breaks only if **every** numeric fiber is poisoned. (b) "Survives restart via the WAL" did not reproduce — after `POST /v1/admin/snapshot` and a restart, reduce works again, because the DHOOM snapshot round-trip normalizes the non-finite away. The degradation window is until the next snapshot, not permanent.

*What remains:* an authenticated write can silently store a non-finite that round-trips lossily (infinity in, null out — invisible in query results, hard to locate) and temporarily breaks variance-gated analytics on bundles with exactly two numeric fibers. Input-validation hardening.

*Fix.* Reject non-finite numeric literals at the parse boundary (`if !n.is_finite() { return Err(...) }` in `parse_literal`) and independently at the storage boundary in the numeric-field coercion path, so the JSON/REST and INGEST writers are covered too. Make the variance filter distinguish "column dropped for zero variance" from "column contains a non-finite value" — the current 422 blames the schema for what is one bad row.

---

### L7 — No zeroization of key material, and `Debug` derived over raw keys

`src/crypto.rs:196` — *downgraded from MEDIUM during verification*

`zeroize` is not a dependency of this crate (it appears in `Cargo.lock` only transitively via `aes-gcm-siv`/`elliptic-curve`), and a repo-wide grep for `zeroize|Zeroize` returns zero hits in Rust source. `GaugeKey`, `FieldTransform::{Opaque,Indexed,Probabilistic}`, `IntegrityKey`, and the `[u8;32]` seed from `GaugeKey::random_seed` (`:519`) are all left in freed heap on drop, and `GaugeKey` is cloned per query (`src/bundle.rs:1525`, `:2882`), multiplying abandoned copies. Separately, `#[derive(Debug, Clone)]` on `FieldTransform` (`:28`) and `GaugeKey` (`:196`) means any `{:?}` on a schema prints raw AES keys, and `BundleSchema` holds `pub gauge_key: Option<GaugeKey>` (`src/types.rs:420`).

*Downgraded* because no path from any caller reaches the impact. The live leak the `Debug` derive would enable does not exist today: no `{:?}` formats a schema or `GaugeKey` anywhere in `gigi_stream.rs`, and the nearest candidate — `engine.create_bundle(schema).unwrap()` at `:12820` — is safe because `create_bundle` returns `io::Result<()>` (`src/engine.rs:1316`), so the unwrap's Debug output is an `io::Error`. The in-memory hygiene half is strictly secondary to H10: the same keys already sit in cleartext in `gigi.wal` and the Tigris bucket, so a core dump discloses nothing an attacker with disk access does not already have — **and will stay that way until H10 is fixed.** Real hardening with a genuine ordering dependency.

*Fix.* Add `zeroize = { version = "1", features = ["zeroize_derive"] }` and wrap key bytes in `#[derive(ZeroizeOnDrop)] struct KeyBytes([u8; 32]);` used by Opaque/Indexed/Probabilistic/IntegrityKey and by the seed locals in `resolve_seed` (`src/parser.rs:10134`) and the CREATE_BUNDLE handler. Replace the `Debug` derives with manual impls printing the variant name and `"[redacted]"`.

---

### L8 — CORS default is wildcard while the code's own docs say restrictive

`src/bin/gigi_stream.rs:1266` — *downgraded from MEDIUM during verification*

`build_cors_layer` (`:1235`) matches on `GIGI_CORS_ORIGIN`; `fly.toml`'s `[env]` block does not set it, so production takes the `Err(_)` arm, which is `AllowOrigin::any()` with `allow_headers` including `x-api-key`. The doc comment at `:1234` says *"unset → restrictive (same-origin only, no CORS headers)"* and the call-site comment at `:16602` repeats it. The code does the opposite.

*Downgraded* because no arm sets `.allow_credentials`, and GIGI auth is header-based (`X-API-Key` / `Authorization`), never cookie-based — so a cross-origin page cannot borrow a victim's credentials. On `gigi-stream.fly.dev` the only endpoints an attacker page can read cross-origin are the ones `auth_middleware` already serves anonymously: `/v1/health` (`:1361`) and `/v1/public/gql` (`:1373`), both readable by plain curl. The marginal disclosure is nil. The "full database read/write from a visited web page" outcome is real only in open mode (L2) or on a localhost dev instance — a different deployment shape.

*Fix.* Make the `Err(_)` arm return `CorsLayer::new()` with no `allow_origin` so unset means no CORS headers, matching the docs; keep `*` strictly opt-in. If the demo pages need wildcard, scope it to `/v1/public/gql` rather than the whole router. Fix the two comments — a config trap where both comments assert the opposite of the code is how this survives review.

---

### L9 — `/v1/health` returns 200 "ok" when the engine lock is unavailable

`src/bin/gigi_stream.rs:1653`

The handler deliberately uses `try_read` to avoid blocking, but its `Err` arm — taken exactly when the engine is unavailable — returns `StatusCode::OK` with `status: "ok"`, zeroed `bundles`/`total_records`, and `loading: Some(true)`. That is a false-healthy signal and a false data-loss signal in the same response, reachable whenever a writer holds the lock (e.g. during the snapshot of M2), a window in which no read can be served.

*The original live verification of this was wrong and is retracted:* during the M2/H7 wedge the 200s came from the `Ok` arm (only a reader held the lock, so `try_read` succeeded and real counts were returned), and once workers were exhausted health stopped responding entirely — so `fly.toml`'s 3 s `[checks.readiness]` would have failed and Fly would have drained the machine. The "platform never notices" impact does not hold for that scenario. What survives is the narrow correctness nit: the `Err` arm reports "ok" for a state in which no query can be served, and reports zero records for a database that has not lost any.

*Fix.* Return 503 with a distinct status (`"degraded"` / `"lock_unavailable"`) from the `Err` arm and keep 200 for the `Ok` arm only. If a contention blip should not flap the machine, gate the 503 on consecutive `try_read` failures rather than a single miss — but do not report "ok", and do not report zero counts, for a state that is neither.

---

### L10 — Unauthenticated `/v1/health` leaks global bundle and record counts

`src/bin/gigi_stream.rs:1643`

`auth_middleware` returns early for `/v1/health` (`:1361-1363`) and `readiness_middleware` also exempts it (`:1293`), so no credential is needed. The handler returns `bundles: engine.bundle_names().len()` and `total_records: engine.total_records()` — engine-wide totals with no namespace filtering, in contrast to `list_bundles` (`:1853-1861`), which does filter by claims.

*Impact.* An anonymous observer polls it: a step in `bundles` reveals a private bundle was created; the slope of `total_records` reveals the write rate into private bundles, including the encrypted `jg_kv` chat store. Aggregate only — no record contents.

*Fix.* Return only `{status, uptime_secs}` on the unauthenticated liveness path; that is all Fly's `[checks.readiness]` consumes. Move counts to the already-authenticated `/v1/metrics` and namespace-filter them there.

---

### L11 — Path-guard errors echo the server's absolute containment root

`src/pathguard.rs:90` (surfaced at `src/bin/gigi_stream.rs:13205-13212`)

An authenticated caller sends `INGEST b FROM 'nope.npz' FORMAT NPZ;` to `/v1/gql`. `resolve_ingest_source` (`src/ingest.rs:452-464`) maps a NotFound into `IngestError::FileNotFound(candidate)` where candidate is `canonical_root.join(screened)` (`src/pathguard.rs:181-185`), and everything else into `SourceNotContained { detail }` carrying the full `Display` including the root (`:69-96`). Those render at `src/ingest.rs:318`/`:324`, become a String at `src/parser.rs:11582-11592`, and reach the client unmodified. With `fly.toml`'s `GIGI_INGEST_DIR=/data/ingest` the caller sees `INGEST: source file not found: /data/ingest/nope.npz`, and success vs NotFound are distinguishable — a file-existence probe inside the ingest root. Live in production: the block is `#[cfg(feature = "gauge")]`, and `Dockerfile:11` builds with `halcyon`, which is `["lattice", "gauge"]`.

*Impact.* Disclosure only, not access — the guard itself is correct. All of `contain()` (`:114-225`) was read: the lexical `Prefix`/`RootDir`/`ParentDir` screen plus the canonical-to-canonical `starts_with` check does block traversal, drive prefixes, and symlink escapes.

*Fix.* Keep the rich `Display` for the server log; map `PathGuardError` to a fixed client-facing string ("ingest source not available under the configured root") at the HTTP boundary in the INGEST dispatch block and the EMIT executor. Do not vary the message by failure kind.

---

### L12 — Flow fit cache key includes an attacker-controlled `sigma_floor_epsilon`

`src/bin/gigi_stream.rs:4694` — *impact narrowed during verification*

Every brain flow endpoint accepts optional `sigma_floor_epsilon: Option<f64>` (`:5396`, `:8302`, `:8438`) and `CacheKey::build` puts `eps.to_bits()` in the key with no normalization, no clamping, and no zeroing when `fit_mode == Isotropic` — where the docstring at `:5389-5394` says the value is ignored. Varying the float per request always misses the cache (`:7902`) and re-runs `compute_fit_data` (`:7946`).

*Narrowed* because the per-request cost is exactly one ordinary cold fit, with no super-linear amplification, on a bundle the tenant provisioned themselves; and the f64 is not the enabling weakness — `fields` is hashed in incoming order (`:4688-4691`, intentionally, per its comment) and `fit_mode` is also in the key, so any caller can miss the cache indefinitely using parameters that legitimately belong there. Fixing the epsilon would not close the bypass. What remains is churn against the 50-entry random-eviction flow cache (`:381-384`, ~10 MB per `FullFitResult` at n=384), which evicts **other bundles'** entries — a modest cross-tenant degradation.

*Fix.* Normalize before the key: force `None` when `fit_mode == Isotropic`, clamp to a documented range, quantize to fixed precision. Same treatment as M5.

---

### L13 — Hand-rolled constant-time comparison without an optimization barrier

`src/credentials.rs:186`

`verify_credential` (`:181`) correctly routes the HMAC tag check through a branch-free accumulator rather than `==`, and the pre-tag early-returns (`:170-176`) are on public metadata exactly as its doc comment claims. `src/integrity.rs:286` does the right thing a different way via `mac.verify_slice(...)`. The nit: `constant_time_eq` is plain safe Rust with nothing preventing LLVM from recognizing accumulate-then-compare and rewriting it with an early exit, and `subtle` — already in `Cargo.lock` transitively — is not a direct dependency.

Two facts make this weaker still, and are recorded rather than hidden. `verify_credential` has **zero callers** outside `src/credentials.rs` (a grep for `verify_credential` and `credentials::` across `src/` returns nothing else), so it is wired to no route today. And the comparator that *is* on the auth hot path is a different function this finding does not cover — `src/bin/gigi_stream.rs:1562`, used by `auth_middleware` at `:1407`, which early-returns on length mismatch (`:1563-1565`) and so leaks the API key's length by timing. That is a standard, generally-accepted trade-off, and it is the same length disclosure M1 hands out for free over HTTP, so it changes nothing operationally.

*Fix.* Promote `subtle` to a direct dependency and replace both copies of `constant_time_eq` (`src/credentials.rs:186` and the twin in `src/threshold.rs`) with `a.ct_eq(b).into()`, which carries the barrier in the type. Delete the duplicate so there is one implementation; consider folding `src/bin/gigi_stream.rs:1562` into the same helper.

---

### L14 — No timeout, concurrency-limit, or body-limit layer; GQL reads run synchronously under the engine lock

`src/bin/gigi_stream.rs:16591`

The router's entire middleware stack is auth, namespace enforcement, rate limiting, readiness, and CORS (`:16591-16603`). `tower-http` is compiled with `features = ["cors"]` only (`Cargo.toml:143`) and `tower` is a dev-dependency for `ServiceExt::oneshot` in tests (`:355-363`), so `TimeoutLayer`, `RequestBodyLimitLayer`, and `ConcurrencyLimitLayer` are not available to add without a manifest change. Meanwhile the read path takes `state.engine_read()` (`:13398`) and calls `execute_gql_with_exists` — a fully synchronous, potentially unbounded computation — directly in the async handler at `:13412`, with only three `spawn_blocking` calls in the whole 21 524-line binary (`:10135`, `:12380`, `:16654`).

*Impact.* This is the structural multiplier on nearly every finding above: nothing can cut short a runaway query, cap how many run at once, or keep `/v1/health` responsive when workers saturate. It converts "slow query" into "machine restart" on a 4-worker single-node deployment. Correctly filed LOW — it is not an attack on its own.

*Fix.* Enable `tower-http`'s `timeout` and `limit` features; add `TimeoutLayer` (tighter on `/v1/public/gql`), `RequestBodyLimitLayer`, and a `ConcurrencyLimitLayer` sized well under `worker_threads`. Move the heavy analytic verbs (SPECTRAL, CURVATURE FULL, the brain flow endpoints, the ML endpoints) onto `spawn_blocking` with cancellation, and clone or snapshot what they need so the engine read lock is not held across compute.

---

# UNVERIFIED LEADS

These are open questions, contested claims, or areas a pass explicitly did not reach. None is a finding. They are listed so the next pass has a worklist and so a reader can see the edges of what was checked.

**U1 — `COVER … EXCLUDING IN <bundle>` as an anonymous private-bundle oracle. Contested; cheapest to settle.**
`validate_public_stmt`'s `S::Cover { bundle, .. }` also discards `excluding` (`src/parser.rs:3303-3316`, field at `:317-326`). One pass traced `apply_excluding_in_filter` as called only from `parser::execute`'s COVER arm (`src/parser.rs:10968`) while the HTTP read path for a non-virtual bundle uses `execute_gql_on_store_read`, which ignores `excluding` entirely — concluding it is **latent**. A second pass asserted it is **live**, and that anonymous `COVER stations ALL EXCLUDING IN jg_kv;` runs a full internal `COVER jg_kv ALL` over the private decrypted bundle, yielding a name-existence oracle (explicit error `EXCLUDING IN bundle 'X' does not exist.` at `src/parser.rs:10026-10029`), a base-key intersection oracle, and an anonymous full-scan DoS amplifier. Neither confirmed at runtime. **One anonymous curl settles it.** Either way the H1 fix should cover `excluding`, because unifying the COVER read path onto `parser::execute` later would make it live.

**U2 — `src/spectral_interior.rs` has six `partial_cmp().unwrap()` sites in production code with no test module.** Its only caller is gauge-gated, and `halcyon` pulls in `gauge`, so it is compiled into the production image. No request was constructed that reaches those comparators with a NaN. Unresolved rather than clean.

**U3 — `src/lattice_delegation.rs` (517 lines) contains no randomness call at all.** That is unusual for a lattice re-encryption scheme and deserves someone confirming that noise/smudging genuinely is not required at that layer rather than merely absent. The file received only a structural skim for RNG and comparison patterns.

**U4 — `src/ratchet.rs`'s `hkdf_step` chain (`:189`) was not reviewed for forward-secrecy correctness.**

**U5 — Does the snapshot writer persist gauge keys the way the WAL does?** `gauge_key` does not appear in `src/mmap_bundle.rs`, but it was not verified that the snapshot path serializes schemas through a different route. If it does not, H10's blast radius includes every `.dhoom` file independently of the WAL.

**U6 — Do the stack-overflow aborts (H2) leave the WAL or a snapshot partially written?** Worth a dedicated pass precisely because H2 makes the abort trivially repeatable by anyone.

**U7 — `SharededSpectralGapRequest.k_max` / `k_neighbors` are caller-supplied and unvalidated at `src/bin/gigi_stream.rs:3578-3585`,** but were not traced into the Lanczos kernel. Same shape as H7/H9.

**U8 — Body-carries-the-bundle-name routes not individually traced.** `tx_write` (`:17067`) takes its bundle from the request body and has the same missing-claims shape as H4; the `transactions` feature is enabled in the production image. The gauge `http::build_router` handlers' `persist:true` write path was also not reviewed. `/v1/quantum_cohomology/*`, `/v1/wish`, `/v1/lattice`, `/v1/gauge_field` were identified by shape, not traced.

**U9 — Deployment configuration is not observable from the repository.** Whether `GIGI_JWT_SECRET`, `GIGI_API_KEY`, `GIGI_RATE_LIMIT`, and `GIGI_TRUST_PROXY` are set via `flyctl secrets` was inferred from `fly.toml`, not observed. Whether the Tigris bucket `gigi-snapshots` is private was not confirmed (`flyctl storage create` defaults to private; only the name was observed, not the ACL). **Three commands — `flyctl config env`, `flyctl secrets list`, and a bucket ACL check — resolve all of it,** and they gate the urgency of M4 and of every tenant-token finding.

**U10 — Not reviewed at all:** `src/bin/gigi_edge.rs`, `src/bin/gigi_server.rs`, the WAL replay and snapshot internals in `src/wal.rs`/`src/dhoom.rs` beyond the schema encode/decode path, durable-growth ratios per write, and the feature-gated routers (`patterns` `/hunt`, `causal_states`, `imagine`, `sharded/*`) beyond route registration and handler signatures.

---

# FIX ORDER

Ordered by what an attacker can do today with what they already have, not by how interesting the defect is.

### Now — anonymous, no credentials, one request

1. **Parser depth guard (H2).** One `depth` field on the `Parser` struct, checked at entry to every recursive production, placed **before** verb dispatch. This is first because it needs no credentials, it is one POST, it kills the process rather than degrading it, and it re-fires faster than a Fly restart plus WAL replay. It is also the cheapest fix on this list and closes three known sites at once.
2. **Walk the condition tree in `validate_public_stmt` (H1).** Anonymous read of every private bundle on the box, with a working character-by-character extraction oracle and a bundle-name enumerator. Second only because it costs data rather than availability, and availability was already lost at #1. Cover `excluding` in the same change (U1) so the contested case is closed regardless of how it resolves.
3. **Bound the regex cache and set a `RegexBuilder` size limit (H3).** Anonymous memory kill at ~200 requests, measured. Same tier as #1; ordered after it only because the depth guard is a smaller diff.
4. **Set `GIGI_RATE_LIMIT` and `GIGI_TRUST_PROXY` (M4).** A config change, not a code change — and it meters #1, #2, #3, and most of the MEDIUM tier while the real fixes land. Do this in the same deploy as #1 rather than waiting. Confirm with `flyctl config env` first (U9).

### Next — authenticated but untrusted tenant

5. **Statement-level and body-level authorization (H4, H5, H6, M1, M2).** One shared fix rather than five: a `state.bundle_for(&claims, name)` accessor that is the only way to obtain a `BundleRef` in the HTTP layer, with `engine.bundle()` made crate-private; a `get_bundle_names(&Statement)` walker feeding `allows_bundle` before GQL dispatch; claims threaded into the WebSocket command handler; and an owner-only `route_layer` over an `/v1/admin/*` sub-router. Grouped because patching them individually is how the next body-carried bundle name gets missed — the point is to make forgetting impossible, not to fix five call sites. Confirm the JWT precondition (U9) first; if no non-owner tokens exist yet, this drops in urgency but not in priority, because it must land **before** the tenant mint goes live.
6. **Input bounds on the ML and brain surface (H7, H8, H9).** Cap `k`, `fields.len()`, `n_steps`, `burn_in`, `n_samples` at their existing validators; add `overflow-checks = true` to `[profile.release]`; move the solves to `spawn_blocking` and stop holding the engine read guard across compute. The lock-hold is the part that turns a bad request into a whole-service outage, so it matters more than any individual cap.

### Scheduled migration — no remote path, highest diligence cost

7. **Replace `mix_hash` with HKDF-SHA256 in the key derivation (H11), and stop persisting derived keys in the WAL (H10).** These are last on the remote-exploitability ordering and first on the "what will an external cryptographer find" ordering. Neither is reachable over HTTP; both are wire-format breaks requiring a versioned gauge-key marker and a re-key of existing encrypted bundles, so they need a migration plan rather than a hotfix. Schedule them now and say so — a spec that calls for HKDF (`GIGI_ENCRYPTION_SPEC.md`) while the code ships an invertible 64-bit mixer is the single hardest item in this document to explain after someone else finds it. `EncryptionSeedSource::Random` means re-derivation alone is insufficient; a KEK path is required. L7 (zeroization) has a genuine ordering dependency on H10 and should ride the same change.

### Then — hardening

8. M3 (make `drop-field` refuse or reindex on encrypted bundles; de-panic `decrypt_value`), M6/M7/M8 (default and cap every unbounded row/body path; fix the two comments that assert the opposite of their code), M5/L12 (quantize cache keys), L14 (enable the tower-http `timeout`/`limit` features and add the three missing layers), then the remaining LOW items. L1, L2, L8, and L9 are each a few lines and can ride any deploy; L1 and L8 especially, since in both cases a comment currently tells a reviewer the mitigation is already in place.

---

# COVERAGE NOTES

The five scoped passes' own accounts of what they reached and what they did not, reproduced verbatim and unedited. Read these before treating any silence in this document as an all-clear.

## 1. auth-and-allowlist

> Swept: the full middleware stack (auth_middleware 1353-1479, namespace_enforcement_middleware 1493-1525, parse_bundle_segment 1529-1556, constant_time_eq 1562-1571, rate_limit_middleware 1575-1625, readiness_middleware 1288-1300, build_cors_layer 1235-1283), verify_gigi_token + GigiClaims 60-167, every .route()/.merge() call in the router build at 16205-16604 including all cfg-gated blocks, validate_public_stmt + public_gql_query 12543-12677, the gql_query dispatch chain 12679-13430 (bundle-less arms, topology/ingest bypasses, needs_write, virtual, read), execute_gql_with_exists 13590-13631 and the COVER arm of execute_gql_on_store_read, the parser's Statement enum + FilterCondition + Condition + parse_cover + parse_filter_condition_list + parse_single_filter, apply_excluding_in_filter and its call sites, QueryCondition::matches in src/bundle.rs, seed_from_hex in src/crypto.rs, and the deployed configuration in fly.toml + Dockerfile (features "kahler imagine sharded transactions patterns causal_states wish halcyon post_kahler_phase1", GIGI_PUBLIC_BUNDLES set, GIGI_RATE_LIMIT unset, GIGI_EMIT_DIR unset).
>
> Answers to the four posed questions. (a) No route is registered outside the auth layer — every .route() and the gauge .merge() precede the .layer() chain at 16591, and axum applies layers to all routes added so far, so nothing escapes; the only intentional bypasses are /v1/health (1361) and /v1/public/gql (1373). The real gap is not a missing auth layer but a missing AUTHORIZATION layer: findings 2, 3, 4 and the /v1/divergence note are all routes that authenticate correctly and then never consult GigiClaims. (b) Yes, bypassable — but not by any of the string tricks. The ';' guard at 12628 is over-strict, not under-strict (a ';' inside a quoted literal produces a false rejection, which fails closed), and it does not matter either way because gigi::parser::parse returns exactly one Statement and gql_query re-parses the same string from the same serde_json::Value — there is no split between what was validated and what executes. Verb-prefix confusion does not apply: dispatch is a match on a typed enum, not a string prefix, and is_keyword (2253) compares with eq_ignore_ascii_case, so case and whitespace normalize identically on both sides. public_gql_query validates query.trim() while gql_query re-reads the untrimmed value; the only divergence a leading non-ASCII whitespace char could cause is a parse error in the second call, which fails closed. The bypass is structural, inside a single well-formed statement: the validator matches Statement::Cover { bundle, .. } and the `..` discards where_conditions, where FilterCondition::Exists carries a second, unchecked bundle name. Concrete working string: COVER stations ALL WHERE EXISTS (COVER jg_kv WHERE key CONTAINS 'sess') — traced end to end in finding 1. (c) No, the allowlist is not enforced on every read path — Exists.cover_bundle is the live hole. I explicitly checked the other second-bundle carriers and they are safe on this endpoint: Pullback and Join are their own Statement variants and fall to the wildcard reject arm; SELECT's Condition enum (parser 1764-1768) has no subquery form; Emit is rejected as a variant and GIGI_EMIT_DIR is unset anyway. COVER ... EXCLUDING IN <bundle> is ALSO unchecked by the validator but I could not trace it to execution over HTTP: apply_excluding_in_filter is only called from parser::execute's COVER arm (parser 10968), and the HTTP read path for a non-virtual bundle uses execute_gql_on_store_read instead, which ignores `excluding` entirely — so I reported it as latent inside finding 1's fix rather than as an exploitable finding. (d) The API key IS compared in constant time (constant_time_eq at 1562, called at 1407); it leaks length via the early return, which is standard and not worth a finding. I found no path that logs or echoes the key — error bodies are the fixed string "Invalid or missing API key", and the logger records GQL statement text and error_msg, not headers. The query-string acceptance in finding 7 is an exposure-to-external-logs issue, not an in-process leak.
>
> Not reached: I did not audit the WebSocket handlers' post-upgrade message loop (ws_handler / ws_dashboard_handler / ws_bundle_dashboard_handler bodies) for per-message bundle authorization — the upgrade itself is gated, but whether a subscribe message can name a foreign bundle after upgrade is unverified. I did not review src/pathguard.rs against the INGEST GIGI_INGEST_DIR containment claim, the transactions tx_write body-bundle path, the gauge http::build_router handlers' persist:true write path, or the crypto/integrity/ledger/*_delegation modules — those fall outside the auth-and-allowlist scope assigned here and to sibling clones. All findings are traced by reading; I did not build or run the binary, so no finding was confirmed against a live server. Nothing in the repository was modified.

## 2. input-validation-panics

> Swept, with live verification against the real release binary. I ran target/release/gigi-stream.exe instances on scratch ports with GIGI_PUBLIC_BUNDLES set and confirmed findings 1, 2, 3, 5, and 6 end-to-end with actual HTTP requests, not just by reading code; finding 4 was confirmed through target/debug/gigi.exe, which links the identical gigi::parser. All test processes were stopped and no repo file was modified.
>
> Files read in full or systematically grepped: src/bin/gigi_stream.rs (21,524 lines — route table, auth middleware including the /v1/public/gql bypass at 1373, validate_public_stmt, public_gql_query, gql_query, execute_gql_with_exists, health, the ML handlers, and an unwrap/expect audit filtered to non-test code below line 17796); src/parser.rs (17,911 lines — every parse_ function enumerated, all recursive-descent productions, parse_literal, parse_usize, tokenize, with test code below line 13884 excluded); all 11 modules under src/ml/; src/patterns/http.rs, src/gauge/http.rs, src/transactions/http.rs, src/discrete/pk_http.rs, src/geometry/pk_http.rs, src/dials.rs; Cargo.toml profiles and features; Dockerfile for the production feature set.
>
> Negative results worth recording. Every partial_cmp().unwrap() sort comparator I could reach turned out to be inside a #[cfg(test)] module (geometry/memory.rs, geometry/wasserstein.rs, lattice/topology/truncated_icosahedron.rs, sharded/execution.rs, stream_shared.rs, vector_cache.rs); the production ML comparators all use unwrap_or(Ordering::Equal) and are NaN-safe. Extreme numeric parameters on the public verbs were handled correctly: COVER FIRST/SKIP 1e30 and 1e309, SPECTRAL MODES 0 and 1e30, CURVATURE and INTEGRATE all returned clean responses with the server alive — `n as usize` saturates rather than wrapping, and parse_usize rejects NaN because NaN >= 0.0 is false. distances.last().unwrap() at gigi_stream.rs:5853 is properly guarded by an is_empty check above it. reduce with k=0, k=99999999, and k=usize::MAX all clamped correctly; only the cluster path overflows.
>
> Not reached, and I would not claim these are clean. The gigi-encrypt modules (crypto.rs, integrity.rs, ledger.rs, mlkem_delegation.rs, lattice_delegation.rs, pairing_delegation.rs) were outside this clone's scope and I did not look at them. src/spectral_interior.rs has six partial_cmp().unwrap() sites in production code with no test module, and its only caller is gauge-gated — `halcyon` pulls in `gauge` so it is compiled into the prod image, but I could not construct a request that reaches those comparators with a NaN, so I am flagging it as unresolved rather than as a finding. I fuzzed reduce, cluster, scan, and changepoints but not solve, factorize, infer, prescribe, or circulation. WebSocket and subscription handlers were only grepped, not exercised. WAL replay and snapshot paths, and the sharded/transactions/discrete/geometry pk_http routes, were not tested live. I also did not determine whether the stack-overflow aborts can leave the WAL or a snapshot partially written on restart — worth a dedicated pass, since finding 1 makes that abort trivially repeatable by anyone.

## 3. resource-exhaustion

> Swept for CLONE 3's remit (resource exhaustion / amplification) across: the full route table and middleware stack in src/bin/gigi_stream.rs (21,524 lines) — every `.route()` registration, the auth/namespace/rate-limit/readiness/CORS layer chain, and the presence-or-absence of body-limit, timeout, and concurrency layers; the public GQL surface (`validate_public_stmt` + every allowlisted verb's executor arm); all request structs whose fields become loop bounds, allocation sizes, or cache keys (`n_samples`, `n_steps`, `burn_in`, `top_k`, `fields`, `bandwidth`, `sigma_floor_epsilon`, `limit`/`offset`); the caching layer (src/vector_cache.rs, src/caches/single_flight.rs, the flow/vector/morse cache key builders); the analytic kernels reachable from HTTP (src/spectral.rs, src/curvature.rs, src/geometry/generative_flow.rs, src/geometry/forms.rs, src/discrete/pk_http.rs, src/stream_shared.rs); and the deployment shape (fly.toml env + [[vm]], Dockerfile feature set — `kahler imagine sharded transactions patterns causal_states wish halcyon post_kahler_phase1` are all ON in production, so every cfg-gated endpoint cited above is live).
>
> Checked and found NOT exploitable: `vector_search`'s `top_k` (src/bundle.rs:3901 truncates after sorting rather than pre-allocating, so a huge top_k costs nothing extra); `CURVATURE` (src/curvature.rs:12 reads precomputed Welford field stats, O(n_fields)); `/v1/bundles/{name}/persistence`, which correctly caps its O(n²) MST at 4000 points (src/discrete/pk_http.rs:46-58) — that is the pattern the other paths should follow; COVER's `FIRST`/`SKIP` are pushed into `filtered_query_ex` rather than applied post-materialization.
>
> Costs are derived from the code (allocation sizes, loop trip counts, flop counts) plus the codebase's own documented latency figures (vector_cache.rs:243-244 "~4 s at N=10 000, D=384"; gigi_stream.rs:6899 "35 s at N=10k, D=384"). I did not run the server, execute any request, or measure wall-clock — no timing claim here is a measurement, and I avoided inventing seconds where the code does not document them. I also could not inspect production bundle contents, so the magnitude of the `field_index_graph` quadratic (finding 3) depends on the real index cardinality of `stations`/`sensors`/`chembl`/`tetmesh_demo`, which should be checked directly; the absence of any cap is a code fact regardless.
>
> Not reached: the WebSocket surface (`/ws`, `/v1/ws/{name}/dashboard`, the broadcast channel map in `StreamState.channels`) — worth a dedicated pass for per-connection state growth and unauthenticated subscribe amplification; the feature-gated routers I only spot-checked (`gigi::gauge::http::build_router`, `/v1/transactions/*`, `/v1/patterns` + `/hunt`, `/v1/wish`, the `sharded/*` bundle routes — `SharededSpectralGapRequest.k_max`/`k_neighbors` are caller-supplied and unvalidated at gigi_stream.rs:3578-3585 but I did not trace them into the Lanczos kernel); src/bin/gigi_edge.rs and gigi_server.rs; the WAL/snapshot internals in src/wal.rs and src/dhoom.rs (I traced only the HTTP entry points, not durable-growth ratios per write); and the crypto modules, which are other clones' remit. No files were modified.

## 4. crypto-implementation

> Scope was the gigi-encrypt implementation only, read-only, no files modified. Fully read: src/crypto.rs (1871 lines — all non-test code), src/integrity.rs, src/credentials.rs, src/ledger.rs, src/wal.rs schema encode/decode (1400-1660), and the encryption-relevant handlers in src/bin/gigi_stream.rs (auth 315-334 and 1353-1472, CREATE_BUNDLE/ROTATE_KEY 12770-12920, app-bundle bootstrap 15705-15890, Tigris push 16830-16840). Swept the whole repo for RNG choice, zeroize/subtle usage, and `==` on secrets.
>
> Two findings are backed by runnable proof-of-concept, not reasoning alone. Both scripts are in the scratchpad at C:\Users\nurdm\AppData\Local\Temp\claude\C--Users-nurdm-OneDrive-Documents-gigi\b37c5552-3973-44ff-8ddd-a1bb403cae11\scratchpad\ (poc_kdf.py, poc_affine.py) and reproduce against a fresh os.urandom seed each run.
>
> Things I checked and found CLEAN, so they need no work:
> - RNG choice is a CSPRNG everywhere it matters. `getrandom` for the master seed (crypto.rs:521), AEAD nonces (crypto.rs:677, mlkem_delegation.rs:162), Gaussian noise (crypto.rs:735), and Shamir field elements (threshold.rs:318); `rand_core::OsRng` in mlkem_delegation.rs:129/148 and pairing_delegation.rs:228/244. The `SmallRng` xorshift64* instances across src/gauge/ and src/geometry/ are lattice-physics samplers with no key role — correct separation.
> - No nonce/IV reuse. `aead_encrypt` draws a fresh 96-bit nonce per call and the cipher is AES-GCM-SIV (RFC 8452), which is nonce-misuse-resistant by construction, so even forced repetition degrades to determinism rather than catastrophe. Rekeys derive fresh keys, so no cross-rekey nonce collision.
> - HKDF domain separation is correct where HKDF is actually used: integrity.rs:101 (salt "gigi-integrity-v1"), mlkem_delegation.rs:155 (KDF_INFO, also bound as AAD), credentials.rs:117 (versioned separator plus length-prefixed fields, so no encoding ambiguity between QueryClass variants).
> - Error messages do not distinguish wrong-key from corrupt-data — `aead_decrypt` (crypto.rs:694) collapses every failure into `Err(())`.
> - The seed itself is never logged, never `Serialize`-derived, and the raw GQL statement text is not written to logs; `EncryptionSeedSource::Hex` does not reach any log sink. The leak in finding #1 is the *derived* keys via the WAL, not the seed via logs.
> - Affine order preservation (`scale` sampled from [0.1, 10.0], always positive, so ciphertext ordering equals plaintext ordering) is a real information leak but is explicitly disclosed in GIGI_GEOMETRIC_ENCRYPTION_SPEC.md:119, so I excluded it per the brief.
>
> Not reached, and worth a separate pass: src/lattice_delegation.rs (517 lines) and src/pairing_delegation.rs (496 lines) got only a structural skim for RNG and comparison patterns, not a full read — lattice_delegation.rs contains no randomness call at all, which is unusual for a lattice re-encryption scheme and deserves someone confirming that noise/smudging genuinely is not required at its layer rather than merely absent. I also did not review src/ratchet.rs's `hkdf_step` chain (ratchet.rs:189) for forward-secrecy correctness, did not audit the snapshot writer in src/mmap_bundle.rs for the same key-persistence problem finding #1 identifies in the WAL (I confirmed `gauge_key` does not appear in that file, but did not verify the snapshot path serializes schemas through a different route), and ran no build or test suite.

## 5. data-boundary-leaks

> Read-only review; no file was modified and no request was made to the live service — every claim above is traced in source.
>
> What I swept: the full HTTP surface assembly in src/bin/gigi_stream.rs (routes 16207-16583, middleware stack 16585-16603) and each of the four middlewares (auth 1352, namespace 1493, rate limit 1573, readiness 1287); every use site of GigiClaims (only 1509, 1853, 1913 — that grep is the load-bearing evidence for the tenancy finding); validate_public_stmt (12543) and public_gql_query (12609) end to end, including the compound-statement split and the ShowBundles special case; token verification (136) and constant_time_eq (1556); build_cors_layer (1235) against fly.toml's actual [env] block; the logging pipeline (observability.rs Logger::emit 413, LogIngester::run 956, LogConfig::default 173, query_start 534) and log_bundle_writer (15910) plus the _gigi_* schemas (15559); Tigris pull/push (15522-15552) and the startup paths that call them (16656, 16680, 16834); data-dir and snapshot-dir permissions in engine.rs (506, 623, 2337 — all 0700, correctly handled); WAL schema encode/decode incl. gauge key (wal.rs 1412-1470, 1536-1660); crypto.rs seed_from_hex (890); transparent decrypt on read (bundle.rs 2144, 2740, 2897); pathguard.rs in full; the WebSocket command handler (12046-12200) and both dashboard handlers; and a scan of all 17 `"error": format!` and ~60 `error: format!` sites for record/key/path echo.
>
> Things I checked and cleared, so they are not re-litigated above: JWT verification is sound (hmac verify_slice is constant-time and length-checked; exp==0 rejected); the public endpoint does NOT leak non-allowlisted bundle existence — validate_public_stmt's bundle_ok returns the identical "not exposed" message whether or not the bundle exists, and ShowBundles is answered without calling the executor, so SHOW BUNDLES cannot enumerate private names; the allowlist and engine lookup are both exact-match (HashMap), so no case-folding bypass; the compound-statement `;` split can only over-reject, not under-reject, and the re-parse in gql_query is deterministic on the same string; path traversal via bundle name into snapshots_dir.join(format!("{name}.dhoom")) is NOT reachable — the GQL tokenizer restricts identifiers to [A-Za-z_][A-Za-z0-9_]* (parser.rs 2168) and the JSON create_bundle route requires a tenant's ns__ prefix, which cannot produce a resolvable `..` component; pathguard's two-layer containment is correct; no error path echoes an API key or the JWT secret; data/snapshot dirs are chmod 0700.
>
> What I did not reach: I could not verify runtime configuration of the live deployment — whether GIGI_API_KEY / GIGI_JWT_SECRET are actually set as Fly secrets (if both are unset, auth_middleware 1463-1472 grants owner claims to every anonymous request, which would escalate the CORS finding to critical), and whether the Tigris bucket "gigi-snapshots" is private (flyctl storage create defaults to private, but I only observed the name committed in fly.toml, not the ACL). I did not audit gigi-encrypt's cryptographic primitives themselves (crypto.rs transform construction, integrity.rs, ledger.rs, mlkem_delegation.rs, lattice_delegation.rs, pairing_delegation.rs) — my clone's scope was data-boundary and leakage, and I only touched crypto.rs where key material crosses a persistence or error boundary. I did not exercise the feature-gated surfaces beyond reading their route registration and handler signatures (gauge, patterns, transactions, causal_states, sharded, imagine); note that tx_write (17067) takes its bundle from the request body and has the same missing-claims problem as /v1/gql, so if `transactions` ships enabled it inherits finding 1. I did not trace the ~60 analytics/brain/ML handlers individually for cost — they are all under /v1/bundles/<name>/*, hence namespace-gated, so they are a DoS question rather than a boundary question.
