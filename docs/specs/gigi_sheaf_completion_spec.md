# GIGI Sheaf Completion Engine
## The Sudoku Principle as a Database Operation
### Bee Rosa Davis · Davis Geometric · 2026-03-27

---

## The Math

On a sheaf, if you have sections defined on overlapping open sets
that agree on the overlaps, the gluing axiom guarantees a unique
global section. If a section is defined at every point in a
neighborhood except one, and the surrounding values are consistent
(H¹ = 0), then the missing value is determined by extension.

This is not interpolation. Interpolation guesses. Sheaf extension
is constrained by the algebraic structure of the bundle. The
missing value isn't the "most likely" value — it's the ONLY value
consistent with the surrounding data. Like a Sudoku cell with one
valid option.

The strength of the constraint depends on how many independent
neighboring sections contribute. This is measurable as the
**constraint density** at the missing point.

---

## Bundle Schema: Adjacency Functions

GIGI's COMPLETE verb knows nothing about drugs, tissues, financial
instruments, or geopolitical entities. It knows three things: sections,
neighbors, and consistency. The definition of "neighbor" is declared in
the bundle schema — not compiled into the engine.

Each bundle schema declares **adjacency functions** inline in the
`CREATE BUNDLE` statement. No TOML config files, no external schema —
the adjacency functions live in GQL where they belong.

```gql
CREATE BUNDLE portfolio
  BASE (asset, sector, date)
  FIBER (return, volatility, correlation)
  ADJACENCY same_sector ON sector = sector WEIGHT 0.4
  ADJACENCY temporal ON |date - date| < 30 WEIGHT 0.3
  ADJACENCY correlation ON |correlation| > 0.7 WEIGHT 0.3
```

Adjacency types:

| Type | GQL syntax | Semantics |
|---|---|---|
| Equality | `ON field = field` | Neighbor if same value in `field` |
| Metric | `ON \|field - field\| < radius` | Neighbor if within numeric radius |
| Threshold | `ON \|field\| > value` | Neighbor if fiber value exceeds threshold |
| Graph | `ON GRAPH graph_name HOPS n` | Neighbor if within n hops in named graph |
| Morphism | `ON MORPH source_bundle.key` | Neighbor via cross-bundle join |

Then `COMPLETE ON portfolio` uses those three adjacency functions with
their declared weights. No finance knowledge in the engine. The bundle
schema carries the domain semantics. GIGI's COMPLETE verb is pure
graph math on whatever adjacency structure the schema defines.

The adjacency function IS the connection on the base space. It is:
- **Domain-agnostic** — GIGI's COMPLETE verb is identical code across
  all domains. A pharma bundle defines neighbors by drug class and
  anatomical proximity. A finance bundle uses sector and correlation
  distance. A geopolitics bundle uses geographic adjacency and treaty
  relationships.
- **Explicit** — completion behavior is determined by the schema.
  Nothing inside the engine encodes domain knowledge.
- **Pluggable** — adding a new adjacency dimension is a schema change,
  not an engine change.

All four completion methods consume the same adjacency functions. The
adjacency graph is computed once per COMPLETE call and cached for the
duration of that call.

---

## GQL Verb: COMPLETE

```
COMPLETE ON <bundle>
  [WHERE <filter>]
  [METHOD <method>]
  [MIN_CONFIDENCE <threshold>]
  [WITH PROVENANCE, CONSTRAINT_MAP]
```

### What it does

1. Scans the bundle for base points where one or more fiber
   values are NULL (unmeasured)
2. For each NULL, examines neighboring sections on the bundle
3. Checks local consistency (H¹ = 0 on the neighborhood)
4. If consistent, extends the section to the missing point
5. Computes constraint confidence from the number and agreement
   of constraining neighbors
6. Stores the completed value as a new section with
   `origin = 'sheaf_completed'` (distinct from
   `origin = 'measured'`)

### Generic response format

```
COMPLETE ON <bundle>
  WHERE <field> IS NULL
  METHOD sheaf_extension
  MIN_CONFIDENCE 0.70
  WITH PROVENANCE, CONSTRAINT_MAP
```

```json
{
  "status": "ok",
  "completed": 3,
  "skipped": 7,
  "results": [
    {
      "base_point": { "<dim_1>": "<val_1>", "<dim_2>": "<val_2>" },
      "field": "<target_field>",
      "completed_value": 0.52,
      "confidence": 0.83,
      "uncertainty_band": 0.08,
      "origin": "sheaf_completed",
      "method": "sheaf_extension",
      "constraint_graph": [
        { "neighbor": "<id_1>", "adjacency": "same_sector", "field_value": 0.50, "weight": 0.40 },
        { "neighbor": "<id_2>", "adjacency": "temporal", "field_value": 0.55, "weight": 0.35 },
        { "neighbor": "<id_3>", "adjacency": "correlation", "field_value": 0.51, "weight": 0.25 }
      ],
      "computation": "weighted_section_extension: Σ(wᵢ × vᵢ) = 0.494 → 0.52 (CoV = 0.09, confidence = 0.99)",
      "provenance": "NOT measured. Geometrically implied by N constraining sections with H¹ = 0.",
      "suggested_experiment": "<domain-specific suggestion declared in bundle schema>"
    }
  ],
  "skipped_reasons": [
    { "base_point": { "<dim>": "<val>" }, "reason": "insufficient_neighbors", "constraint_density": 0.31 },
    { "base_point": { "<dim>": "<val>" }, "reason": "inconsistent_neighborhood", "H1": 1 }
  ]
}
```

> **Domain example (MIRADOR / pharmacokinetics):**
> `COMPLETE ON mirador_universe WHERE tissue = 'bone' AND K_barrier IS NULL`
> In this schema, `drug_class` adjacency uses a structural lookup table
> and `tissue` adjacency uses an anatomical proximity graph. The engine
> is unaware of this. The MIRADOR bundle schema declares it.

---

## Completion Methods

### 1. `sheaf_extension` (default)

Extends the section from measured neighbors using the sheaf
gluing axiom. Requires H¹ = 0 on the local neighborhood
(consistency). Refuses to complete if neighbors contradict.

**Constraint sources (in priority order):**

Each row corresponds to an adjacency function declared in the bundle
schema. Weight ranges are schema-level defaults; individual calls may
override weights explicitly.

| Adjacency type | Default weight range | Description |
|---|---|---|
| Same entity, adjacent base-space dimension | 0.10-0.20 | Same entity measured at a neighboring point |
| Same class, same dimension | 0.30-0.40 | Structurally analogous entity at the same location |
| Metric property model | 0.15-0.25 | Prediction from a numeric property via metric adjacency |
| Correlated property | 0.10-0.20 | Known fiber-fiber correlation within the bundle |
| Morphism (external bundle) | 0.20-0.30 | Value pulled from an adjacent bundle via declared morphism |

Weights are normalized to sum to 1.0. The confidence is:

```
confidence = 1 / (1 + CoV²)
```

where CoV = std(weighted predictions) / mean(weighted predictions).
This is bounded [0, 1], equals 1.0 when all neighbor predictions agree
exactly, and is consistent with the C = τ/K confidence scores used
throughout the Davis framework. High agreement = high confidence. One
outlier = lower confidence. Contradictory sources = H¹ ≠ 0 =
completion refused.

### 2. `pullback_inference`

If the target value exists in a related bundle connected by a declared
morphism, pull it back. This reuses the same bundle morphism
infrastructure already used by GIGI's GQL `MORPH` verb — an adjacency
function of type `morphism` in the schema is all that is required.
Confidence is discounted by the morphism quality score declared in the
schema (how well-aligned the external bundle's measurement methodology
is with the target bundle's).

```
COMPLETE ON <bundle>
  WHERE <field> IS NULL
  METHOD pullback_inference
  FROM <source_bundle>
```

> **Domain example (MIRADOR):**
> `COMPLETE ON mirador_universe WHERE drug = 'tedizolid' AND tissue = 'bone'`
> `METHOD pullback_inference FROM pbpk_bundle`
> — Pulls a PBPK-predicted value across the `chembl_id` morphism
> declared in the MIRADOR schema. The engine doesn't know what PBPK
> means. The schema declares the morphism and its quality discount.

### 3. `spectral_interpolation`

Uses the spectral decomposition of the bundle's connection
Laplacian. Missing values are reconstructed from the dominant
eigenmodes of the data — the "low-frequency" structure of the
bundle. High-frequency (noisy) components are discarded.

This is the fiber bundle analogue of Fourier interpolation:
reconstruct the missing signal from the strongest harmonic
components. Appropriate when many values are missing but the
overall structure is smooth.

```
COMPLETE ON <bundle>
  WHERE <field> IS NULL
  METHOD spectral_interpolation
  MODES 5
  MIN_CONFIDENCE 0.60
```

### 4. `parallel_transport` *(v2 — requires connection coefficients)*

Uses the fiber bundle connection to transport a known section
along a path in the base space. The holonomy (failure of parallel
transport to return to its starting value after a loop) measures
curvature — which becomes the confidence penalty.

This method is geometrically the most precise but computationally
the most demanding: it requires computing connection coefficients
over the transport path, which in turn requires sufficient data
density along that path.

```
COMPLETE ON <bundle>
  WHERE <field> IS NULL
  METHOD parallel_transport
  ALONG <base_space_path>
  FROM <anchor_base_point>
```

Save for v2. Build `sheaf_extension` and `pullback_inference` first.

---

## Confidence Classification

| Confidence | Sudoku analogy | Interpretation | Action |
|---|---|---|---|
| 0.90-1.00 | 8/9 neighbors filled | Near-certain. One valid value. | Treat as fact pending confirmation. |
| 0.70-0.89 | 6-7/9 neighbors | High confidence. Value tightly constrained. | Strong hypothesis. Prioritize for cheap validation. |
| 0.50-0.69 | 4-5/9 neighbors | Moderate. Multiple consistent values possible. | Directional. Design experiment to disambiguate. |
| 0.30-0.49 | 2-3/9 neighbors | Low. Weakly constrained. | Exploratory. Don't make clinical decisions on this. |
| < 0.30 | 0-1/9 neighbors | Insufficient. Not completable. | Skipped. `MIN_CONFIDENCE` prevents return. |

---

## Discovery Queries

### "Where should I look next?"

```
COMPLETE ON <bundle>
  WHERE origin = 'sheaf_completed'
    AND confidence > 0.85
  RANK BY impact_score DESC
  FIRST 20
  WITH SUGGESTED_EXPERIMENT
```

Returns the 20 highest-confidence unmeasured values with the
greatest domain impact (impact_score is a bundle schema field —
clinical significance in pharma, market exposure in finance,
instability index in geopolitics). Each one is an actionable
target. The geometry tells you where to dig.

### "What's geometrically impossible that we thought was true?"

```
CONSISTENCY ON <bundle>
  WHERE H1 > 0
  RANK BY contradiction_severity DESC
  WITH PROVENANCE
```

Returns every place where the data in the bundle contradicts itself.
These aren't gaps — they're errors. One of the source measurements
is wrong. The geometry tells you which ones to recheck.

### "Where is data most needed?"

```
COMPLETE ON <bundle>
  WHERE <target_field> IS NULL
  RANK BY constraint_density ASC
  FIRST 10
```

Returns the base points with the LEAST constraint — the most open
Sudoku cells. These are the gaps where a new measurement would
provide the most information to the bundle. This is optimal
experimental design from the geometry.

### "If I measured X, what else would it determine?"

```
PROPAGATE ON <bundle>
  ASSUMING <dim_1> = '<val_1>' AND <dim_2> = '<val_2>' AND <field> = <value>
  SHOW newly_determined
```

Returns every other NULL in the bundle that would become
completable if you added this one measurement. Shows the cascade:
one measurement can trigger 5, 10, 50 sheaf completions elsewhere
in the bundle. This is the Sudoku domino effect — fill one cell,
three more become forced.

---

## Domain Boundary: What Stays In vs Out of GIGI

### In GIGI (domain-agnostic, pure math):

1. **H¹ check** — consistency across neighbors, z-score outlier
   detection. The z-score threshold is configurable per bundle
   (schema declares `H1_THRESHOLD 3.0`), but the check itself is
   engine code.
2. **Confidence formula** — `1 / (1 + CoV²)`. Invariant. Not
   configurable. This is a mathematical identity.
3. **Origin tracking** — `measured` vs `sheaf_completed`. Pure
   bookkeeping. Every completed value carries its provenance forever.
4. **Constraint graph** — which neighbors contributed, with what
   weights, through which adjacency function. This is the debugging
   tool: when a completion turns out wrong, trace back WHY the
   geometry predicted what it did. Bad neighbor? Missing adjacency
   dimension? Planted contradiction that slipped past H¹?

### Out of GIGI (domain-specific, declared in bundle schema):

1. **What "neighbor" means** — the ADJACENCY clauses in CREATE BUNDLE.
   The engine evaluates them; the domain expert declares them.
2. **What z-score threshold catches contradictions** — configurable
   per bundle. Pharma may use 3.0 (conservative). Finance may use
   2.5 (aggressive). The engine applies whatever the schema says.
3. **What "impact" means for ranking gaps** — `impact_score` is a
   fiber field in the bundle, not a concept the engine understands.
   A pharma bundle defines it as clinical significance. A finance
   bundle defines it as market exposure. GIGI just sorts by it.

---

## Integrity Rules

### Completed values are ALWAYS marked

Every sheaf-completed value carries `origin = 'sheaf_completed'`
permanently. It is NEVER silently promoted to
`origin = 'measured'`. A client can always distinguish between
experimentally verified data and geometrically implied data.

### Measured values always override

If an experimental measurement arrives for a cell that was
previously sheaf-completed, the measurement replaces the
completion. The old completion is archived with the measured
value logged as validation:

```json
{
  "event": "completion_validated",
  "base_point": { "<dim_1>": "<val_1>", "<dim_2>": "<val_2>" },
  "field": "<target_field>",
  "completed_value": 0.52,
  "measured_value": 0.49,
  "deviation": 0.03,
  "within_uncertainty_band": true,
  "note": "Sheaf completion confirmed within ±0.08 band"
}
```

This is how the Sudoku Principle self-validates. Every completed
value is a prediction. Every subsequent measurement is a test.
Over time, the completion accuracy rate is empirically measurable.

### Cascade recomputation

When a measured value arrives that changes a section, all
downstream sheaf completions that depended on it are
automatically recomputed. If the new measurement contradicts
previous completions (moves them outside their uncertainty band),
those completions are invalidated and the confidence is
recomputed. The webhook system fires `coherence_change` events
for any affected downstream values.

### H¹ ≠ 0 blocks completion

If the local neighborhood is inconsistent (contradictory source
data), the completion is refused entirely. The system never
produces a value by averaging contradictory inputs. Instead, it
flags the inconsistency via `CONSISTENCY` and waits for the
contradiction to be resolved by additional measurement or source
correction.

---

## The Sudoku Principle as a Service

The COMPLETE verb creates a natural feedback loop.

When a client queries a bundle for a base point that has no measured
value, instead of returning `"no data"`, GIGI can automatically:

1. Run COMPLETE on the requested cell
2. Return the sheaf-completed value with `origin: 'sheaf_completed'`
   and a confidence score
3. Include `suggested_experiment`: the specific measurement that would
   convert this completion to a measured value
4. When the experiment runs and the measurement arrives, it replaces
   the completion — and cascades via PROPAGATE to recompute any
   downstream completions that depended on it

The bundle gets denser with every measurement. Denser bundles enable
more completions. More completions surface more experimental targets.
This is the loop:

```
query → no measured value
      → COMPLETE → prediction + confidence
      → suggested_experiment
      → measurement arrives
      → COMPLETE validated or corrected
      → PROPAGATE → downstream completions unlock
      → loop
```

This works for any fiber bundle. A pharma bundle returns a predicted
penetration ratio and suggests a surgical sample assay. A finance
bundle returns a predicted instrument price and suggests a market
discovery event. A geopolitics bundle returns a predicted relationship
strength and suggests a field observation.

GIGI is not a database that stores facts. It is a machine that
computes all facts consistent with what it has been told, and tells
you exactly what to measure next to make it more certain.

---

## The Deeper Insight

Traditional databases store what you know.
The Sudoku Principle computes what you MUST know but haven't
measured yet.

The fiber bundle doesn't guess. It constrains. The sheaf axioms
don't produce "most likely" values — they produce the unique
values consistent with the surrounding data. When the confidence
is high, the completed value is not a prediction. It is a
theorem with an error bar.

This changes the economics of science. Today, discovering a new
pharmacokinetic fact requires: grant application ($50K-500K),
IRB approval (6 months), patient enrollment (1-2 years),
sample collection, mass spectrometry, statistical analysis,
peer review, publication. Total: $500K and 3 years per fact.

With sheaf completion: the fact is implied by existing data.
The experiment is still worth running (to confirm), but the
discovery has already happened. The experiment becomes
validation, not exploration. And the system tells you which
experiments to run first (highest confidence, highest
clinical impact, longest cascade of downstream completions).

The scientific method doesn't change. The order changes.
Discover first. Validate second. The fiber bundle tells you
what's true. The lab confirms it.

---

*GIGI · The Sudoku Principle · Davis Geometric · 2026*
*The data knows more than the scientists who collected it.*
