# gigi Stability Guarantees

gigi ships behind feature flags, and not every flag carries the same
contract. This document declares which surfaces are safe to build a
product on, which are safe to experiment with, and which you should
pin by commit hash. The math under the hood is settled work; the
caveats here are about **API stability and ops experience**, not about
whether the geometry is right.

## The three tiers

gigi sorts every public surface into one of three tiers.

**Production-stable.** Semver is the contract. Breaking changes to
request shape, response shape, error codes, or persisted on-disk
formats happen only on major version bumps (`0.x` → `0.y` until
1.0; `1.x` → `2.x` after). Behavior is locked by tests that run on
every commit. Build a product on these surfaces with confidence.

**Research with caveats.** Semver is still the contract for existing
fields and verbs, but the surface is still growing. Minor version
bumps may add new fields to response bodies, new optional query
parameters, new verbs, or new observables. Existing fields stay
where they are and mean what they meant. Build a product on these
surfaces; just don't pattern-match exhaustively on enums or assume
response objects are closed.

**Research-grade.** Under active iteration. Breaking changes are
allowed within minor versions. Request shapes, error codes, and
persisted formats may shift between commits. If you depend on a
research-grade surface in production, pin a commit hash and capture
your own integration tests against that pinned copy. The math
underneath is not in question; the surface is.

## Feature-flag stability

| Feature flag      | Tier                    | Stability promise                                                                                                         | When to depend on it                                                              |
|-------------------|-------------------------|---------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------|
| *(no feature)*    | Production-stable       | Core CRUD, query, range, aggregate, join, schema evolution, stats, curvature, spectral gap. Persisted format frozen.      | Any production use — this is what `gigi-stream.fly.dev` runs.                     |
| `kahler`          | Production-stable       | 12-layer Kähler substrate (L1–L12). Brain primitives at `/v1/bundles/{name}/brain/*`. Additive over base — byte-identical when off. | Production reads and writes against brain primitives.                              |
| `lattice`         | Production-stable       | Graph-topology primitive (vertices + signed edges + face-cycle table). Canonical buckyball constructor. `LATTICE` GQL verb. | Building gauge or Halcyon-shaped applications.                                    |
| `gauge`           | Production-stable       | Group-erased connection algebra. SU(2) ships with full math; U(1) and Z_N compile but panic at use site.                  | SU(2) work in production. Avoid U(1) / Z_N at runtime until those constructors land. |
| `halcyon`         | Production-stable       | Composite (`lattice + gauge`). SU(2) Yang-Mills thermalization. Read routes live. Snapshot verb (Part V) lands next.       | SU(2) thermalization research with semver caveats on Part V snapshot format.       |
| `causal_states`   | Production-stable       | TV / Hellinger / KL diagnostics, update commutator, regime classifier. 36/36 validation gates green.                       | Causal-state diagnostics in production.                                            |
| `transactions`    | Production-stable       | Atomic sheaf commits — 2PC + recovery + the three GIGI invariants (cocycle bound, K-monotone, connection-coherent).        | Multi-bundle ACID writes in production.                                            |
| `imagine`         | Research with caveats   | IMAGINE / WALK extrapolation. RK4 geodesic integrator (T11 green) and gauge-equivariance proof (T12 green). Safety envelope `K_ceiling = 4.0`. | Forward-projection in production with awareness that new provenance fields may appear. |
| `wish`            | Research with caveats   | Boundary-value-problem geodesic verb. `WishOutcome` trichotomy (Granted / Unreachable / Indeterminate). Relaxation solver default; Shooting available. Gates W1–W5 green. | BVP geodesics in production with awareness that solver selection and metric trait surface may grow. |
| `sharded`         | Research-grade          | Atlas-cover sharding (ChartId, ShardId, Atlas, Transition, SpectralRegime, Clean Finger Move). Types defined; execution bodies are `todo!()` until Phase B. 13 test gates passing on the API skeleton (Penguins T13 cross-chart loop included). | Experimentation only. Pin a commit hash before depending on it.                    |
| `patterns`        | Research-grade          | DEFINE PATTERN / HUNT / DROP PATTERN / SHOW PATTERNS / EXCLUDING IN. Phase 1 ships parser-only — grammar stable, executor incomplete. | Experimentation only. Pin a commit hash before depending on it.                    |

## Per-endpoint stability

The endpoint table below is the practical view: which routes you can
call from a production client today, and which routes will reshape
under you.

### Production-stable endpoints

These honor semver. Request and response shapes are frozen within the
current major version.

| Endpoint                                       | Method    | Notes                                                                  |
|------------------------------------------------|-----------|------------------------------------------------------------------------|
| `/v1/health`                                   | GET       | Unauthenticated. Returns `{"status":"ok"}`.                            |
| `/v1/bundles`                                  | GET, POST | List bundles / create bundle from schema.                              |
| `/v1/bundles/{name}/insert`                    | POST      | Insert one record.                                                     |
| `/v1/bundles/{name}/update`                    | POST      | Update one record by key.                                              |
| `/v1/bundles/{name}/delete`                    | POST      | Delete one record by key.                                              |
| `/v1/bundles/{name}/get`                       | POST      | Point lookup by key.                                                   |
| `/v1/bundles/{name}/query`                     | POST      | Predicate query. Response carries curvature and confidence per record. |
| `/v1/bundles/{name}/range`                     | POST      | Range scan.                                                            |
| `/v1/bundles/{name}/join`                      | POST      | Cross-bundle join.                                                     |
| `/v1/bundles/{name}/aggregate`                 | POST      | Group-by aggregation.                                                  |
| `/v1/bundles/{name}/stream`                    | GET (WS)  | WebSocket subscription.                                                |
| `/v1/bundles/{name}/stats`                     | GET       | Bundle stats (record count, Welford moments, curvature, spectral gap). |
| `/v1/bundles/{name}/schema`                    | GET       | Current schema.                                                        |
| `/v1/bundles/{name}/curvature`                 | GET       | Aggregate curvature.                                                   |
| `/v1/bundles/{name}/spectral`                  | GET       | Spectral-gap connectivity diagnostic.                                  |
| `/v1/bundles/{name}/consistency`               | GET       | Cocycle-bound consistency check.                                       |
| `/v1/transactions/begin`                       | POST      | Open a multi-bundle transaction.                                       |
| `/v1/transactions/{id}/commit`                 | POST      | Two-phase commit.                                                      |
| `/v1/transactions/{id}/rollback`               | POST      | Roll back.                                                             |
| `/v1/causal_states/commutator`                 | POST      | TV / Hellinger / KL commutator with regime classifier.                 |
| `/v1/bundles/{name}/brain/attend`              | POST      | Attention primitive.                                                   |
| `/v1/bundles/{name}/brain/confidence`          | POST      | Per-record confidence.                                                 |
| `/v1/bundles/{name}/brain/episodic`            | POST      | Episodic recall.                                                       |
| `/v1/bundles/{name}/brain/explain`             | POST      | Per-record explanation.                                                |
| `/v1/bundles/{name}/brain/focus`               | POST      | Focused subbundle.                                                     |
| `/v1/bundles/{name}/brain/inpaint`             | POST      | Field inpainting.                                                      |
| `/v1/bundles/{name}/brain/predict`             | POST      | One-step prediction.                                                   |
| `/v1/bundles/{name}/brain/reconstruct`         | POST      | Trajectory reconstruction.                                             |
| `/v1/bundles/{name}/brain/dream`               | POST      | Anisotropic-flow trajectory.                                           |
| `/v1/bundles/{name}/brain/forecast`            | POST      | Forecast envelope.                                                     |
| `/v1/bundles/{name}/brain/semantic`            | POST      | Semantic neighborhood.                                                 |
| `/v1/bundles/{name}/brain/self_monitor`        | POST      | Self-monitor diagnostic.                                               |
| `/v1/gql` (verbs: INSERT, QUERY, RANGE, JOIN, AGGREGATE, SHOW, BEGIN, COMMIT, ROLLBACK, LATTICE, GAUGE_FIELD) | POST | The base parser surface plus the gauge-theory verbs. |

### Research-with-caveats endpoints

These honor semver for fields that already exist. New fields may be
added in minor versions; new outcome variants may be added to
trichotomy enums in minor versions.

| Endpoint                                       | Method | Notes                                                                                  |
|------------------------------------------------|--------|----------------------------------------------------------------------------------------|
| `/v1/bundles/{name}/brain/imagine`             | POST   | Extrapolation. `ImaginedProvenance` may grow new variants.                             |
| `/v1/bundles/{name}/brain/wish`                | POST   | BVP geodesic. `WishOutcome` may grow variants beyond Granted / Unreachable / Indeterminate. |
| `/v1/gql` Halcyon verbs (GIBBS_SAMPLE, MEASURE_EVERY, SNAPSHOT, ...) | POST | SU(2) thermalization. Snapshot format (Part V) lands next and may iterate. |

### Research-grade endpoints

Surface and behavior may change between commits. Pin before depending.

| Endpoint                                       | Method | Notes                                                                                  |
|------------------------------------------------|--------|----------------------------------------------------------------------------------------|
| `/v1/patterns` (and GQL `DEFINE PATTERN`, `HUNT`, `DROP PATTERN`, `SHOW PATTERNS`, `EXCLUDING IN`) | POST | Parser surface stable; executor incomplete. |
| Any `sharded`-feature endpoints                | —      | Skeleton only. Phase B integrates with `BundleStore`.                                  |

## What does production-stable actually mean

Concretely:

- Request and response JSON shapes do not change within a major
  version. Fields are not removed, renamed, or retyped. New optional
  request fields may be added; clients sending the old shape continue
  to work.
- Error codes (HTTP status + `error` string in body) are stable.
- The on-disk persistence format is forward-compatible within a major
  version. A bundle written by `0.1.k` reads on `0.1.k+n`.
- Behavior under fixed inputs is stable to numerical precision pinned
  by the gold-fixture tests in `tests/fixtures/`. Halcyon receipts
  are validated to 256-bit SHA precision.
- Auth model is stable: `X-API-Key` header, optional per deployment
  via `GIGI_API_KEY`. `/v1/health` is always unauthenticated.

What it does **not** mean:

- It does not mean the math will never get sharper. New observables
  and new optional fields land in minor versions. Existing observables
  do not move.
- It does not mean every internal Rust API is frozen. The Rust crate
  surface tracks the same tiers as the HTTP surface; embedded users
  on the production tier get the same semver contract, embedded users
  on a research-grade feature should pin like everyone else.

## How to pin a research-grade feature

If you need `sharded` or `patterns` (or any future research-grade
surface) in something that has to keep working:

1. Pin gigi by commit hash in `Cargo.toml`:
   ```toml
   gigi = { git = "https://github.com/davisgeometric/gigi", rev = "<full-sha>", features = ["sharded"] }
   ```
2. Capture an integration test against your pinned copy. Assert the
   exact request and response shapes your code depends on. Run the
   test on every build.
3. When you bump the pin, run your captured tests first. If they
   break, the surface moved — that is the contract for research-grade.

For research-with-caveats features (`imagine`, `wish`, the Halcyon
GQL surface), pinning a release version is enough; just don't write
exhaustive `match` arms on `WishOutcome` or `ImaginedProvenance`
without a wildcard branch.

For production-stable features, depend on the published version range
as you would any semver-respecting library.

## Authentication and transport

These are stable across all tiers:

- **Header**: `X-API-Key: <key>` on every endpoint except `/v1/health`
  when `GIGI_API_KEY` is set on the server.
- **Error shape**: `{"error": "<message>"}` with a non-2xx HTTP
  status.
- **Content type**: `application/json` for requests and responses;
  `/v1/bundles/{name}/stream` upgrades to WebSocket.
- **Encoding**: UTF-8.

## When tiers change

A feature can graduate from research-grade to research-with-caveats
to production-stable. It cannot move the other way without a major
version bump. Graduations are announced in `CHANGELOG.md` alongside
the version that ships them, and are reflected here on the same
commit.

If you are not sure which tier a surface lives in, the truth is in
this file and in `Cargo.toml` feature comments. If those disagree,
file an issue — `Cargo.toml` and this document are required to stay
in sync.
