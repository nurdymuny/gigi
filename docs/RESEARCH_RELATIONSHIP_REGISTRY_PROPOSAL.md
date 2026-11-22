# Proposal: typed research relationships with evidence

Date: 2026-10-24  
Status: proposed; no engine implementation, ingestion, or deployment in this change.  
Requested by: Bee Rosa Davis.  
Source-review baseline: GIGI commit `35e62c4`; working-tree documents may also contain proposals. No live-engine certification was performed.

## User need

Save a connection once so collaborators and agents can retrieve what connects two ideas, where the evidence is, what assumptions apply, and what remains unproved. Example: [coordinate transformations and Geometry of Sameness](research/coordinate-translations-and-sameness.md).

This requires two related but distinct records:

- Documentary relation: "this example illustrates the mechanism in that manuscript."
- Executable transformation: "this specific map, on this domain, has this Jacobian, metric, inverse, and measured error."

A semantic link is not an executable map. A stored transformation is not proof of semantic equivalence.

## Existing components inspected

| Component | What source shows | Limit for this use case |
|---|---|---|
| [ProvenanceGraph](../src/coherence.rs), around line 694 | Parent/source IDs, backward `why` and forward `implications` graph walks | Edges have no per-edge relationship type, assumptions, proof, or source locator. A dependency walk is not logical entailment. Runtime API availability and persistence need verification. |
| [Bundle provenance field](../src/bundle.rs), around line 755 | Optional provenance graph on a bundle | Does not itself establish restart persistence or shared-agent access. |
| [Structural adjacency](../src/graph/adjacency.rs) | Principal/auxiliary graphs from field relationships | Adjacency alone does not distinguish analogy, specialization, equivalence, and contradiction. |
| [Sharded atlas transitions](../src/sharded/atlas.rs), around line 69 | Chart IDs, Lipschitz estimate, invertibility flag, transition representation; explicitly references Geometry of Sameness | In this source the representation enum is `Identity` or `Placeholder`. Metadata must not be advertised as a general executable nonlinear translator. |
| [Cross-atlas bridges](../src/sharded/cross_atlas.rs) | Bridge transitions and cocycle-check infrastructure | Numerical compatibility infrastructure is not a citation/claim registry. |
| [Pullback join](../src/join.rs) | Matches records through a foreign-key lookup | A pullback join is not the differential-geometric pullback metric J^T h J. |
| [Sheaf completion](../src/sheaf/mod.rs) | Schema adjacency, numeric restriction coefficients, completion machinery | Does not certify prose claims or arbitrary mathematical equivalence. |
| [Marcella verified inference minispec](MARCELLA_GEOMETRIC_INFERENCE_MINISPEC.md) | Related proposal for explicit numeric constraints, evidence, and checked derivations | It is labeled proposed, not implemented. This registry should complement it rather than pretend it has shipped. |

Finding: useful infrastructure exists. No complete typed, evidence-backed research relationship workflow was identified in these inspected components. This is a bounded source review, not a proof that no other module implements any part of the workflow.

## Minimal first implementation

Use ordinary records and joins for a documentary registry before introducing new engine verbs. The following names are a logical schema proposal, not verified GQL syntax:

- `research_sources`: stable ID, author, title, URI or repository path, version/hash, precise locator, access scope, captured date, and `availability`.

  `availability` is one of `available`, `unavailable` (temporarily unreachable),
  `retracted` (withdrawn by its publisher), or `restricted` (this requester lacks
  permission). These produce different outcomes and must not be merged. A
  retraction bears on whether a relation is still supported; a dead link does not;
  a permission failure bears on one requester and must not leak the content or
  imply the source is gone. A version hash fixes *which* version is meant — it
  does not make that version readable by anyone else, so hash-attached and
  collaborator-accessible are separate preconditions for promotion.
- `research_concepts`: stable ID, name, mathematical definition, domain, units where relevant, source IDs.
- `research_relations`: stable ID, subject ID, claimed relation type, object ID, plain-English statement, assumptions, scope, evidence IDs, established altitude, lifecycle, `asserted_by`, `asserted_by_is_agent`, `reviewed_by`, `reviewed_by_is_agent`, version, supersedes/retracts ID.
- `research_checks`: relation ID, method, input artifact hashes, expected outcome, actual outcome, execution environment, date, result, and whether the evidence is a proof, numerical experiment, or editorial comparison.

### What is claimed, what is established, and where the record stands

These are three independent fields. Collapsing any two of them reintroduces the
failure the registry exists to prevent.

**1. Claimed relation** — what the asserter says the relationship is. Every type's
direction is fixed here and read strictly as *subject → relation → object*.
Traversal is not implementable until this table is settled, because walking an
edge the wrong way inverts its meaning silently.

| Type | Reads as | Direction |
|---|---|---|
| `illustrates` | subject is an example of what object asks or asserts | directed |
| `uses_definition` | subject **uses** object's definition | directed |
| `special_case_of` | subject is a special case of object | directed |
| `generalizes_under` | subject generalizes object, under stated conditions | directed |
| `derived_from` | subject is derived from object | directed |
| `uses_same_mechanism` | both use the same named mechanism | **symmetric** |
| `equivalent_under` | equivalent, under stated conditions | **symmetric** |
| `contradicts_under` | mutually inconsistent, under stated conditions | **symmetric** |

Symmetric types must not be stored or traversed as if directed. Directed types must
never be walked backwards to manufacture a converse: *A `uses_definition` B* does
not license *B `used_by` A* as a stored relation.

Anyone may assert any of these. Asserting a strong type establishes nothing.

**2. Established altitude** — what the evidence actually supports, using the
existing Davis-Geometric vocabulary. This is *earned from evidence records*, never
derived from the claimed type:

| Altitude | What must exist to earn it |
|---|---|
| `NONE` | Asserted with no evidence record. |
| `INCIDENCE` | At least one source with a precise locator showing the two things co-occur, or that one is an instance or illustration of the other. |
| `RESONANCE` | The same named mechanism is identified **and located** in both subject and object. Naming a mechanism on one side only does not qualify. |
| `STRUCTURAL` | Named domains, maps and hypotheses, plus a `research_checks` row carrying a derivation or proof reference, or a numerical check **whose falsifying control was run and did fail**. A check that passes under every variant tests nothing. |

A relation may be `equivalent_under` at `INCIDENCE`. That gap is legitimate and
is the registry's most useful query: *show every relation whose claim outruns its
evidence.*

**Altitude never renders alone.** It must always appear with its `evidence_kinds`
and `altitude_scope`. `STRUCTURAL` earned by a proof reference and `STRUCTURAL`
earned by a numerical check are not the same claim: a numerical check with a
working negative control establishes something about the cases it tested, not a
universal theorem, and the scope field is where that limit is stated. A retrieval
surface that shows the altitude badge without the scope is non-conforming.

**3. Evidence kind** — `source`, `proof`, `numerical`, `editorial`. These are
**unordered**. A numerical check is not a weaker proof and a proof is not a
stronger source; they answer different questions. Never rank, average, or
minimise across them.

**4. Lifecycle** — `proposed`, `current`, `superseded`, `retracted`. Ordered in
time, not in strength, and orthogonal to evidence kind. Retraction is a lifecycle
event, not a low score.

Never infer equivalence from embedding similarity, a shared word, matching outputs
on a fixture, or a path through the provenance graph. Never infer priority or
historical influence from a mathematical resemblance.

### Path results

A query that traverses more than one edge returns a **path**, never a derived
relation. The path carries every edge's own claimed type, altitude, evidence kind
and lifecycle, unreduced. It additionally carries one composition state:

- `composition_unchecked` — the default, and the only state a machine may assign.
- `compatible_under: [conditions]` — a reviewer checked the hypotheses and recorded
  the conditions under which the edges compose.
- `incompatible: [reason]` — the hypotheses conflict. Recorded, not hidden.

Listing the union of two edges' assumptions does not make them compatible. One
edge may require a map to be invertible while the next discards information; the
union of those assumptions is contradictory and a bag of strings will not say so.
Promoting a path to a single stored relation is a separate, human-reviewed write.

Never infer equivalence from embedding similarity, a shared word, matching outputs on a fixture, or a path through the provenance graph. Never infer priority or historical influence from a mathematical resemblance. Qualified relations do not compose unless their hypotheses and domains have been checked.

## Initial record

The polar note touches two manuscripts with different strengths, so it becomes two
records rather than one. Bundling them would dilute the stronger claim.

```json
[
  {
    "id": "DG-REL-DAVISMANIFOLD-PULLBACK-001",
    "subject": "davis-manifold-metric-construction",
    "object": "classical-pullback-metric-JtJ",
    "claimed_relation": "uses_definition",
    "statement": "The Davis Manifold metric construction uses the classical pullback metric J^T h J.",
    "scope": "The shared construction only. Not a claim about the manuscript's detection or error-budget results.",
    "assumptions": ["full column rank J", "target metric h positive definite"],
    "evidence": ["DG-REF-COORDINATE-SAMENESS-001"],
    "evidence_kinds": ["source"],
    "established_altitude": "RESONANCE",
    "altitude_basis": "Mechanism named and located on both sides: classical reference, and the manuscript near line 445 at review time.",
    "altitude_scope": "Holds for the metric construction as written in that passage. No numerical or proof check performed.",
    "lifecycle": "proposed",
    "lifecycle_note": "Manuscript locator is a local path. A hash would fix the version; it would not make the manuscript readable by a collaborator. Both are required before promotion to current.",
    "asserted_by": "Astra", "asserted_by_is_agent": true,
    "reviewed_by": null, "reviewed_by_is_agent": null, "reviewer_decision": "unreviewed"
  },
  {
    "id": "DG-REL-POLAR-SAMENESS-001",
    "subject": "classical-polar-coordinate-map",
    "object": "geometry-of-sameness-translation-geometry-relationship",
    "claimed_relation": "illustrates",
    "statement": "An exact coordinate transformation illustrates how translated descriptions can share a consistent geometric measurement rule.",
    "scope": "Explanatory example. Does not establish the manuscript's general equivalence or error-transfer claims.",
    "assumptions": ["r > 0", "local injective angular chart", "Euclidean target metric"],
    "evidence": ["DG-REF-COORDINATE-SAMENESS-001"],
    "evidence_kinds": ["source"],
    "established_altitude": "INCIDENCE",
    "altitude_basis": "A source with a precise locator shows the example illustrates the question. No mechanism is located on the manuscript side.",
    "altitude_scope": "The polar chart on r > 0 only.",
    "lifecycle": "proposed",
    "asserted_by": "Astra", "asserted_by_is_agent": true,
    "reviewed_by": null, "reviewed_by_is_agent": null, "reviewer_decision": "unreviewed"
  }
]
```

### Adversarial fixture for gate 2

The registry's own codebase supplies a better negative case than anything
synthetic, and one that is already live in a public API.

```json
{
  "id": "DG-REL-PULLBACK-HOMONYM-001",
  "subject": "gigi-pullback-join",
  "object": "differential-geometric-pullback-metric",
  "claimed_relation": "uses_same_mechanism",
  "statement": "ASSERTED, NOT ESTABLISHED. Both are called 'pullback'.",
  "scope": "Adversarial fixture. src/join.rs:17 (also POST /v1/bundles/{name}/join) computes a fiber product over a key. The pullback metric f*h = J^T h J is precomposition along a map. Both uses of the word are correct in their own setting; they are different constructions.",
  "assumptions": [],
  "evidence": [],
  "evidence_kinds": [],
  "established_altitude": "NONE",
  "altitude_basis": "No mechanism located on both sides. A shared term is not a mechanism.",
  "altitude_scope": null,
  "lifecycle": "proposed",
  "asserted_by": "fixture", "asserted_by_is_agent": true,
  "reviewed_by": null, "reviewed_by_is_agent": null, "reviewer_decision": "unreviewed"
}
```

Gate 2 passes only if this record stays at `NONE`/`proposed` under retrieval, and
if nothing — embedding similarity, the shared token, or a provenance walk — promotes
it. A separately justified connection between the two remains permitted; it would
have to earn its altitude through evidence like any other.

These records live in this proposal only; none has been inserted into GIGI.

## Optional executable map layer

Later, attach source/target chart IDs, coordinate order and units, forward/inverse map, valid domain and branch, target metric, Jacobian implementation/version, rank/conditioning checks, and error tolerances. Use allowlisted operators or reviewed adapters rather than executing arbitrary expressions from reference material. Keep analytical guarantees separate from sample-based estimates.

The polar fixture must produce det J = r, J^T J = diag(1,r^2), and a round trip on the chosen chart. It must refuse an inverse at r = 0 and expose the angular branch requirement. This should reuse atlas facilities only after their executable capabilities are verified.

## Acceptance gates

1. Retrieve the initial relationship from the plain-English questions in the reference note, with its source and scope visible.
2. Store `DG-REL-PULLBACK-HOMONYM-001`. Retrieval must keep it at altitude `NONE`
   and lifecycle `proposed`. No shared term, embedding neighbourhood, or provenance
   path may raise either field.
3. Remove a supporting source: the relation's established altitude must drop to
   whatever the remaining evidence still earns, recomputed rather than retained.
   Retracting a source is a distinct event from deleting one and must cascade to
   every relation resting on it.
4. Retract a relation: ordinary answers must exclude it as current evidence while audit history remains accessible.
5. Restart the engine and query from a second authorized client: IDs, evidence, versions, and status must persist unchanged.
6. Enforce source access rules across clients; "shared" means accessible to authorized collaborators, not automatically public.
7. Assert `equivalent_under` with no evidence record. Altitude must remain `NONE`.
   Claiming a strong relation type must never raise established altitude; only an
   evidence record does that. The query "claim outruns evidence" must list it.
8. For an executable-map extension, remove the radial factor from the metric and confirm the polar fixture fails. A zero-residual sample must not establish a global theorem.
9. **Composition, with a genuinely blocked fixture.** The first edge produces an
   **unoriented axis**; the second requires a **uniquely determined directed
   arrow** and supplies no further orientation information. The needed datum does
   not exist, so the composite is not merely degraded — it cannot be formed. The
   path must return both edges intact with `composition_unchecked`, must not emit
   a derived relation, and must not present the union of assumptions as if jointly
   satisfiable. A reviewer marking it `incompatible` with that reason must persist.

   *Why this fixture and not the obvious one:* "invertible, then lossy" composes
   perfectly well; the composite is simply not invertible. Using it would have
   produced a gate that passes whether or not the engine detects incompatibility.
   The orientation case is the S¹ → ℝP¹ double cover: the 2:1 map admits no
   continuous section, so the orientation is absent rather than degraded.
10. **Contradiction, bounded and in both halves.** Version one detects
    **structured** conflicts only: the same quantity, object, domain and
    conditions, carrying incompatible asserted values or properties. Those must
    surface. Two relations that differ only because their stated domains differ
    must **not** surface — a test that accepts either outcome tests neither.
    Prose claims are routed to a *possible conflict, for review* queue and are
    never auto-labelled `contradicts_under`. No general mathematical contradiction
    detector is proposed or implied.
11. **Agent attribution.** A relation asserted by an agent and reviewed by nobody
    must be distinguishable at retrieval from one a human reviewed, without
    inspecting history.
12. **Three source failures, three outcomes.** Mark a source `retracted`, a second
    `unavailable`, and a third `restricted` for the requesting client. Retraction
    must flag every dependent relation for review. Unavailability must not change
    any altitude. Restriction must affect only that requester, must not reveal the
    content, and must not report the source as missing or withdrawn.
13. **Direction.** Store `A uses_definition B` and query from both ends. B's view
    must show it is *used by* A, and must not present a stored converse relation.
    A symmetric type must return identically from either end.

## Revisions

**2026-10-24, after review by Bee and Claude.** Three corrections to the first
draft, all of which removed a premature summarisation:

1. **Review status was a single ladder; it is now two independent fields.**
   `source_supported` and `numerically_checked` are different *kinds* of evidence,
   not different strengths, so they cannot be ordered, ranked or minimised across.
   Retraction was mixed into the same enum and is now a separate lifecycle state.
   A proposal to reduce a path to its "minimum status" was dropped for the same
   reason: the minimum of an unordered set is not defined.
2. **Union of assumptions replaced by an explicit composition state.** Listing two
   edges' assumptions does not make them compatible — one edge may require
   invertibility while the next discards information. Paths now carry
   `composition_unchecked` / `compatible_under` / `incompatible`, and a machine may
   only assign the first.
3. **Altitude is earned from evidence, never mapped from the relation type.** The
   first draft proposed `equivalent_under` → STRUCTURAL. That would let anyone
   promote a claim by choosing a strong word, which is the exact failure the
   altitude tags exist to prevent. Claimed type and established altitude are now
   independent, and the gap between them is a first-class query.

The holonomy framing for unchecked composition is left **exploratory and
unrecorded**. Unjustified composition is not by itself a flat connection; that
reading needs defined fibers, transport maps and a meaningful notion of
composition before it is even an `INCIDENCE` claim, and the registry should not
ship a bridge it would have to mark `proposed`.

**2026-10-24, second review.** Four corrections to the implementation contract:

4. **Relation direction was unspecified, and the first record was backwards.** It
   stored *classical pullback metric → uses_definition → Davis Manifold*, which
   asserts the classical construction borrows from the manuscript. Its own
   `statement` field said the opposite, so the record was internally inconsistent.
   Direction is now fixed per type in a table, three types are marked symmetric,
   and traversal is blocked until that table is settled.
5. **The incompatible-composition fixture was not incompatible.** "Invertible then
   lossy" composes fine; the composite is just not invertible. That gate would
   have passed whether or not the engine detected anything — the exact defect this
   document warns about elsewhere. Replaced with unoriented axis → required
   directed arrow, where the needed datum is absent rather than degraded.
6. **Contradiction detection bounded to structured conflicts.** Same quantity,
   object, domain and conditions with incompatible values or properties. Prose
   goes to a review queue. No general mathematical contradiction detector is
   claimed.
7. **Evidence handling tightened.** Altitude never renders without its
   `evidence_kinds` and `altitude_scope`, so `STRUCTURAL` by proof and
   `STRUCTURAL` by numerical check are not confusable. Source `availability`
   separates retracted, unavailable and restricted, which have different
   consequences. A version hash fixes which version is meant and does not grant
   anyone access to it.

## Scope of the first release

Ordinary records and joins supply storage, but a schema alone is not the
capability. The first release covers **saving, searching, inspecting evidence, and
following paths**, with the natural-language surface provided by a retrieval layer
such as Marcella or an application over GIGI — not by new engine verbs. The two
manuscript relations and the pullback fixture are the content to build it against.
Executable nonlinear translators remain a later phase and are not a dependency.

## Rollout

The executable fixture and its current limits are documented in
[`e2e/probes/registry/README.md`](../e2e/probes/registry/README.md). Its regression
checks include failed writes, missing evidence, stable replacement rulings,
retraction-aware paths, bounded typed comparisons, and a read-only restart check.
This fixture implementation does not yet satisfy every proposed capability above,
including requester-specific access control and the executable-map layer.

First review the schema and source-to-claim distinctions with the GIGI maintainers. Next verify storage/query/restart behavior against a freshly built isolated live engine, following AGENTS.md. Only then ingest approved records and expose retrieval to consuming agents. No new theorem-proving or general relationship-discovery capability is implied by this proposal.
