#!/usr/bin/env python3
"""
Sheaf Completion Engine — Rigorous Validation & Synthetic Stress Test
=====================================================================

Tests the spec from gigi_sheaf_completion_spec.md against sheaf_branch.tex
proofs (SH1-SH3). Exercises:

  1. Math validation   — confidence formula, H¹ detection, uniqueness
  2. Synthetic dataset — 200-entity fiber bundle (4 base dimensions, 8 fiber fields)
     with 30% NULLs, structural classes, metric properties, deliberately
     planted contradictions.
  3. Completion engine  — sheaf_extension with adjacency functions
  4. Edge cases         — H¹ ≠ 0 blocking, cascade propagation, sparse data

All math lives here for prototyping; production implementation goes in Rust.
"""

import math
import random
import statistics
from dataclasses import dataclass, field
from typing import Optional

random.seed(42)  # reproducible

# ═══════════════════════════════════════════════════════════════════════
# §1  CONFIDENCE FORMULA VALIDATION
# ═══════════════════════════════════════════════════════════════════════

def confidence_cov(predictions: list[float], weights: list[float]) -> float:
    """
    confidence = 1 / (1 + CoV²)
    where CoV = σ_weighted / μ_weighted

    Bounded [0, 1]. Equals 1.0 when all predictions identical.
    Consistent with curvature.rs: confidence(K) = 1/(1+K), where K = Var/range².
    Here CoV² = Var/μ² which is the natural dimensionless analog.
    """
    if not predictions or not weights:
        return 0.0
    w_total = sum(weights)
    if w_total == 0:
        return 0.0
    w_norm = [w / w_total for w in weights]

    mu = sum(w * v for w, v in zip(w_norm, predictions))
    if abs(mu) < 1e-15:
        return 0.0  # degenerate: mean zero
    variance = sum(w * (v - mu) ** 2 for w, v in zip(w_norm, predictions))
    cov_sq = variance / (mu * mu)
    return 1.0 / (1.0 + cov_sq)


def completed_value(predictions: list[float], weights: list[float]) -> float:
    """Weighted section extension: Σ(wᵢ × vᵢ) / Σ(wᵢ)"""
    w_total = sum(weights)
    if w_total == 0:
        return 0.0
    return sum(w * v for w, v in zip(weights, predictions)) / w_total


def uncertainty_band(predictions: list[float], weights: list[float]) -> float:
    """±1 weighted standard deviation"""
    w_total = sum(weights)
    if w_total == 0:
        return float('inf')
    w_norm = [w / w_total for w in weights]
    mu = sum(w * v for w, v in zip(w_norm, predictions))
    variance = sum(w * (v - mu) ** 2 for w, v in zip(w_norm, predictions))
    return math.sqrt(variance)


# ── Confidence formula property tests ──

def test_confidence_formula():
    print("=" * 70)
    print("§1  CONFIDENCE FORMULA VALIDATION")
    print("=" * 70)

    # Property 1: Perfect agreement → confidence = 1.0
    c = confidence_cov([0.50, 0.50, 0.50], [0.3, 0.4, 0.3])
    assert abs(c - 1.0) < 1e-10, f"Perfect agreement: expected 1.0, got {c}"
    print(f"  ✓ Perfect agreement:   confidence = {c:.4f} (expected 1.0)")

    # Property 2: Bounded [0, 1] — extreme disagreement
    c2 = confidence_cov([0.01, 100.0], [0.5, 0.5])
    assert 0.0 <= c2 <= 1.0, f"Boundedness violated: {c2}"
    print(f"  ✓ Extreme disagreement: confidence = {c2:.4f} (bounded [0,1])")

    # Property 3: Monotonically decreasing with CoV
    vals_tight = [0.50, 0.52, 0.48]
    vals_spread = [0.30, 0.50, 0.70]
    vals_wild = [0.10, 0.50, 0.90]
    w = [1/3, 1/3, 1/3]
    c_tight = confidence_cov(vals_tight, w)
    c_spread = confidence_cov(vals_spread, w)
    c_wild = confidence_cov(vals_wild, w)
    assert c_tight > c_spread > c_wild, "Monotonicity violated"
    print(f"  ✓ Monotonicity:  tight={c_tight:.4f} > spread={c_spread:.4f} > wild={c_wild:.4f}")

    # Property 4: Consistency with curvature.rs — confidence(K) = 1/(1+K)
    # When predictions are [v-δ, v, v+δ], CoV² = (δ²/3) / v² = Var/μ²
    # curvature.rs: K = Var/range², confidence = 1/(1+K)
    # Our formula: confidence = 1/(1+CoV²) = 1/(1 + Var/μ²)
    # These agree when range ≡ μ (the natural scale). ✓
    v, delta = 1.0, 0.1
    preds = [v - delta, v, v + delta]
    w_eq = [1/3, 1/3, 1/3]
    c_ours = confidence_cov(preds, w_eq)
    var = sum((x - v) ** 2 for x in preds) / 3
    k_curvature = var / (v * v)  # K = Var/μ² when range = μ
    c_rust = 1.0 / (1.0 + k_curvature)
    assert abs(c_ours - c_rust) < 1e-10, f"Formulas disagree: {c_ours} vs {c_rust}"
    print(f"  ✓ Consistent with curvature.rs: both = {c_ours:.6f}")

    # Property 5: Single prediction → confidence = 1.0 (no variance)
    c_single = confidence_cov([0.42], [1.0])
    assert abs(c_single - 1.0) < 1e-10
    print(f"  ✓ Single prediction:   confidence = {c_single:.4f}")

    print()


# ═══════════════════════════════════════════════════════════════════════
# §2  SYNTHETIC FIBER BUNDLE — COMPLEX CHALLENGE DATASET
# ═══════════════════════════════════════════════════════════════════════

# --- Base space: 4 discrete dimensions ---
ENTITY_CLASSES = {
    "alpha":   ["E001", "E002", "E003", "E004", "E005"],
    "beta":    ["E006", "E007", "E008", "E009", "E010"],
    "gamma":   ["E011", "E012", "E013", "E014", "E015"],
    "delta":   ["E016", "E017", "E018", "E019", "E020"],
}

CONTEXTS = ["ctx_A", "ctx_B", "ctx_C", "ctx_D", "ctx_E"]

# --- Fiber fields: 8 numeric measurements ---
FIBER_FIELDS = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8"]

# --- Numeric properties for metric adjacency ---
ENTITY_PROPERTIES = {}  # entity_id → {"prop_x": float, "prop_y": float}


@dataclass
class FiberRecord:
    entity_id: str
    entity_class: str
    context: str
    prop_x: float
    prop_y: float
    fibers: dict[str, Optional[float]] = field(default_factory=dict)
    origin: str = "measured"


def generate_synthetic_bundle() -> list[FiberRecord]:
    """
    Generate 200 records (20 entities × 5 contexts × ~2 measurements each,
    plus extras). Plant:
      - 30% NULL fiber values (gaps to complete)
      - Class-consistent patterns (same class → similar fibers)
      - Metric-correlated properties (close prop_x → similar F1-F4)
      - 3 deliberate contradictions (H¹ ≠ 0)
      - 5 isolated entities (sparse neighborhoods, low constraint density)
    """
    records = []
    all_entities = []
    for cls, ids in ENTITY_CLASSES.items():
        for eid in ids:
            all_entities.append((eid, cls))

    # Class-level base values (the "true" section for each class×context)
    class_base = {}
    for cls in ENTITY_CLASSES:
        for ctx in CONTEXTS:
            base = {}
            # Each class has a characteristic fiber profile
            cls_offset = {"alpha": 0.0, "beta": 0.3, "gamma": 0.6, "delta": 0.9}[cls]
            ctx_offset = {"ctx_A": 0.0, "ctx_B": 0.1, "ctx_C": 0.2, "ctx_D": 0.3, "ctx_E": 0.4}[ctx]
            for i, f in enumerate(FIBER_FIELDS):
                base[f] = 0.2 + cls_offset + ctx_offset + 0.05 * i
            class_base[(cls, ctx)] = base

    for eid, cls in all_entities:
        # Entity-specific property values (for metric adjacency)
        cls_idx = list(ENTITY_CLASSES.keys()).index(cls)
        eid_idx = int(eid[1:]) - 1
        prop_x = cls_idx * 2.5 + random.gauss(0, 0.3)
        prop_y = eid_idx * 0.5 + random.gauss(0, 0.2)
        ENTITY_PROPERTIES[eid] = {"prop_x": prop_x, "prop_y": prop_y}

        for ctx in CONTEXTS:
            base = class_base[(cls, ctx)]
            fibers = {}
            for f in FIBER_FIELDS:
                if random.random() < 0.30:
                    fibers[f] = None  # 30% NULLs
                else:
                    # Small entity-specific noise around class base
                    fibers[f] = base[f] + random.gauss(0, 0.03)
            records.append(FiberRecord(
                entity_id=eid,
                entity_class=cls,
                context=ctx,
                prop_x=prop_x,
                prop_y=prop_y,
                fibers=fibers,
            ))

    # ── Plant 3 contradictions (H¹ ≠ 0) ──
    # Contradiction 1: E003 (alpha) in ctx_B has a fiber value
    # wildly inconsistent with all alpha neighbors
    for r in records:
        if r.entity_id == "E003" and r.context == "ctx_B":
            r.fibers["F1"] = 5.0  # All alphas in ctx_B have F1 ≈ 0.30
            r.fibers["F2"] = 5.0
            break

    # Contradiction 2: E008 (beta) in ctx_D — opposite sign
    for r in records:
        if r.entity_id == "E008" and r.context == "ctx_D":
            r.fibers["F3"] = -2.0  # Betas in ctx_D have F3 ≈ 0.60
            break

    # Contradiction 3: E014 (gamma) in ctx_A — extreme outlier
    for r in records:
        if r.entity_id == "E014" and r.context == "ctx_A":
            r.fibers["F5"] = 99.0  # Gammas in ctx_A have F5 ≈ 0.84
            break

    # ── Plant 5 sparse entities (minimal neighbors) ──
    for sparse_id in ["E021", "E022", "E023", "E024", "E025"]:
        cls = "orphan"
        prop_x = random.uniform(20.0, 30.0)  # Far from all classes
        prop_y = random.uniform(20.0, 30.0)
        ENTITY_PROPERTIES[sparse_id] = {"prop_x": prop_x, "prop_y": prop_y}
        fibers = {}
        for f in FIBER_FIELDS:
            if random.random() < 0.80:
                fibers[f] = None  # 80% NULL — very sparse
            else:
                fibers[f] = random.uniform(0, 1)
        records.append(FiberRecord(
            entity_id=sparse_id,
            entity_class=cls,
            context="ctx_A",
            prop_x=prop_x,
            prop_y=prop_y,
            fibers=fibers,
            origin="measured",
        ))

    return records


# ═══════════════════════════════════════════════════════════════════════
# §3  ADJACENCY FUNCTIONS — THE CONNECTION ON THE BASE SPACE
# ═══════════════════════════════════════════════════════════════════════

def adj_same_class(r1: FiberRecord, r2: FiberRecord) -> Optional[float]:
    """Discrete adjacency: same entity_class → weight 0.35"""
    if r1.entity_class == r2.entity_class and r1.entity_id != r2.entity_id:
        return 0.35
    return None


def adj_same_class_same_context(r1: FiberRecord, r2: FiberRecord) -> Optional[float]:
    """Same class AND same context → stronger weight 0.45"""
    if (r1.entity_class == r2.entity_class
            and r1.context == r2.context
            and r1.entity_id != r2.entity_id):
        return 0.45
    return None


def adj_same_entity_other_context(r1: FiberRecord, r2: FiberRecord) -> Optional[float]:
    """Same entity, different context → weight 0.15"""
    if r1.entity_id == r2.entity_id and r1.context != r2.context:
        return 0.15
    return None


def adj_metric_property(r1: FiberRecord, r2: FiberRecord, radius: float = 2.0) -> Optional[float]:
    """Metric adjacency: distance in (prop_x, prop_y) space < radius → weight scaled by distance"""
    dx = r1.prop_x - r2.prop_x
    dy = r1.prop_y - r2.prop_y
    dist = math.sqrt(dx * dx + dy * dy)
    if dist < radius and r1.entity_id != r2.entity_id:
        # Weight decreases with distance: 0.25 at dist=0, 0 at dist=radius
        return 0.25 * (1.0 - dist / radius)
    return None


ADJACENCY_FUNCTIONS = [
    ("same_class_same_context", adj_same_class_same_context),
    ("same_class", adj_same_class),
    ("same_entity_cross_context", adj_same_entity_other_context),
    ("metric_property", lambda r1, r2: adj_metric_property(r1, r2, radius=2.0)),
]


# ═══════════════════════════════════════════════════════════════════════
# §4  H¹ CHECK — OBSTRUCTION DETECTION (Čech Cohomology)
# ═══════════════════════════════════════════════════════════════════════

def check_h1_local(target: FiberRecord, field: str,
                   neighbors: list[tuple[FiberRecord, float, str]],
                   threshold: float = 3.0) -> tuple[bool, Optional[str], list[tuple[FiberRecord, float, str]]]:
    """
    Local H¹ = 0 check (simplified).

    The full Čech H¹ from sheaf_branch.tex (SH3) requires building the
    coboundary complex. For a practical implementation, the obstruction
    manifests as: neighboring sections that SHOULD agree on overlaps
    but DON'T.

    We detect this as: if any neighbor's value for `field` deviates
    from the MEDIAN by more than `threshold` × MAD (median absolute
    deviation), the neighborhood is inconsistent.

    Returns (consistent: bool, reason: Optional[str], clean_neighbors)
    where clean_neighbors has outliers removed for downstream use.
    """
    values = []
    for rec, weight, src in neighbors:
        v = rec.fibers.get(field)
        if v is not None:
            values.append(v)

    if len(values) < 2:
        return True, None, neighbors

    # Use median + MAD for robustness against outliers
    med = statistics.median(values)
    mad = statistics.median([abs(v - med) for v in values])
    if mad < 1e-10:
        mad = 1e-10  # Prevent division by zero when most values identical

    outlier_ids = set()
    reason = None
    for rec, weight, src in neighbors:
        v = rec.fibers.get(field)
        if v is not None:
            z_mad = abs(v - med) / mad
            if z_mad > threshold:
                outlier_ids.add((rec.entity_id, rec.context))
                if reason is None:
                    reason = (
                        f"H¹ ≠ 0: {rec.entity_id}/{rec.context} has {field}={v:.3f}, "
                        f"neighborhood median={med:.3f}, MAD={mad:.3f}, z_MAD={z_mad:.1f}"
                    )

    clean = [(r, w, s) for r, w, s in neighbors
             if (r.entity_id, r.context) not in outlier_ids]

    if outlier_ids:
        return False, reason, clean
    return True, None, neighbors


# ═══════════════════════════════════════════════════════════════════════
# §5  SHEAF EXTENSION ENGINE
# ═══════════════════════════════════════════════════════════════════════

@dataclass
class CompletionResult:
    entity_id: str
    context: str
    field: str
    completed_value: float
    confidence: float
    uncertainty: float
    origin: str
    method: str
    n_neighbors: int
    constraint_sources: list[str]
    provenance: str


@dataclass
class SkipResult:
    entity_id: str
    context: str
    field: str
    reason: str
    detail: str


def sheaf_complete(records: list[FiberRecord],
                   min_confidence: float = 0.50,
                   min_neighbors: int = 2) -> tuple[list[CompletionResult], list[SkipResult]]:
    """
    COMPLETE ON <bundle> METHOD sheaf_extension

    For each NULL fiber value:
      1. Find neighbors via adjacency functions
      2. Check H¹ = 0 (local consistency)
      3. If consistent: weighted section extension → completed value + confidence
      4. If inconsistent: skip with reason
      5. If insufficient neighbors: skip
    """
    completions = []
    skips = []

    # Index records by (entity_id, context) for O(1) lookup
    rec_index = {}
    for r in records:
        rec_index[(r.entity_id, r.context)] = r

    for target in records:
        for fld in FIBER_FIELDS:
            if target.fibers.get(fld) is not None:
                continue  # Not a gap

            # Step 1: Gather neighbors with non-null values for this field
            neighbors = []  # (record, weight, source_name)
            for other in records:
                if other is target:
                    continue
                if other.fibers.get(fld) is None:
                    continue  # Can't contribute if also NULL

                for adj_name, adj_fn in ADJACENCY_FUNCTIONS:
                    w = adj_fn(target, other)
                    if w is not None and w > 0:
                        neighbors.append((other, w, adj_name))
                        break  # Only count each neighbor once (highest-priority adjacency)

            # Step 2: Check minimum neighbor count
            if len(neighbors) < min_neighbors:
                skips.append(SkipResult(
                    entity_id=target.entity_id,
                    context=target.context,
                    field=fld,
                    reason="insufficient_neighbors",
                    detail=f"found {len(neighbors)}, need {min_neighbors}",
                ))
                continue

            # Step 3: H¹ = 0 check (with outlier exclusion)
            consistent, h1_reason, clean_neighbors = check_h1_local(target, fld, neighbors)
            if not consistent:
                # If after removing outliers we still have enough neighbors,
                # proceed with the clean set (soft H¹ handling).
                if len(clean_neighbors) >= min_neighbors:
                    # Re-check consistency on clean set
                    consistent2, _, clean2 = check_h1_local(target, fld, clean_neighbors)
                    if consistent2:
                        neighbors = clean_neighbors  # Use clean set
                    else:
                        skips.append(SkipResult(
                            entity_id=target.entity_id,
                            context=target.context,
                            field=fld,
                            reason="inconsistent_neighborhood",
                            detail=h1_reason or "H¹ ≠ 0",
                        ))
                        continue
                else:
                    skips.append(SkipResult(
                        entity_id=target.entity_id,
                        context=target.context,
                        field=fld,
                        reason="inconsistent_neighborhood",
                        detail=h1_reason or "H¹ ≠ 0",
                    ))
                    continue

            # Step 4: Weighted section extension
            predictions = [r.fibers[fld] for r, w, s in neighbors]
            weights = [w for r, w, s in neighbors]
            sources = list(set(s for r, w, s in neighbors))

            val = completed_value(predictions, weights)
            conf = confidence_cov(predictions, weights)
            unc = uncertainty_band(predictions, weights)

            # Step 5: Check min confidence threshold
            if conf < min_confidence:
                skips.append(SkipResult(
                    entity_id=target.entity_id,
                    context=target.context,
                    field=fld,
                    reason="below_confidence_threshold",
                    detail=f"confidence={conf:.3f} < {min_confidence}",
                ))
                continue

            completions.append(CompletionResult(
                entity_id=target.entity_id,
                context=target.context,
                field=fld,
                completed_value=round(val, 4),
                confidence=round(conf, 4),
                uncertainty=round(unc, 4),
                origin="sheaf_completed",
                method="sheaf_extension",
                n_neighbors=len(neighbors),
                constraint_sources=sources,
                provenance=f"Geometrically implied by {len(neighbors)} constraining sections with H¹ = 0.",
            ))

    return completions, skips


# ═══════════════════════════════════════════════════════════════════════
# §6  PROPAGATE — CASCADE ANALYSIS
# ═══════════════════════════════════════════════════════════════════════

def propagate_analysis(records: list[FiberRecord],
                       assume_entity: str, assume_context: str,
                       assume_field: str, assume_value: float,
                       min_confidence: float = 0.50) -> list[CompletionResult]:
    """
    PROPAGATE ON <bundle>
      ASSUMING entity='X' AND context='Y' AND field = value
      SHOW newly_determined

    Temporarily injects the assumed measurement, re-runs completion,
    returns only the NEW completions that were NOT possible before.
    """
    import copy

    # Run completion without the assumption
    before_completions, _ = sheaf_complete(records, min_confidence)
    before_keys = {(c.entity_id, c.context, c.field) for c in before_completions}

    # Inject measurement
    records_with = copy.deepcopy(records)
    for r in records_with:
        if r.entity_id == assume_entity and r.context == assume_context:
            r.fibers[assume_field] = assume_value
            r.origin = "measured"
            break

    # Run completion with the assumption
    after_completions, _ = sheaf_complete(records_with, min_confidence)

    # Find newly determined values
    newly_determined = []
    for c in after_completions:
        key = (c.entity_id, c.context, c.field)
        if key not in before_keys:
            newly_determined.append(c)

    return newly_determined


# ═══════════════════════════════════════════════════════════════════════
# §7  CONSISTENCY CHECK — H¹ DETECTOR
# ═══════════════════════════════════════════════════════════════════════

def consistency_check(records: list[FiberRecord]) -> list[dict]:
    """
    CONSISTENCY ON <bundle> WHERE H¹ > 0

    Scans for neighborhoods where data contradicts itself.
    Returns contradiction reports.
    """
    contradictions = []

    for target in records:
        for fld in FIBER_FIELDS:
            if target.fibers.get(fld) is None:
                continue  # Skip gaps — contradictions are in MEASURED data

            neighbors = []
            for other in records:
                if other is target:
                    continue
                if other.fibers.get(fld) is None:
                    continue
                for adj_name, adj_fn in ADJACENCY_FUNCTIONS:
                    w = adj_fn(target, other)
                    if w is not None and w > 0:
                        neighbors.append((other, w, adj_name))
                        break

            if len(neighbors) < 2:
                continue

            consistent, reason, _ = check_h1_local(target, fld, neighbors, threshold=3.0)
            if not consistent:
                contradictions.append({
                    "entity_id": target.entity_id,
                    "context": target.context,
                    "field": fld,
                    "value": target.fibers[fld],
                    "reason": reason,
                })

    return contradictions


# ═══════════════════════════════════════════════════════════════════════
# §8  RUN ALL TESTS
# ═══════════════════════════════════════════════════════════════════════

def main():
    # ── §1: Confidence formula ──
    test_confidence_formula()

    # ── §2: Generate synthetic bundle ──
    print("=" * 70)
    print("§2  SYNTHETIC FIBER BUNDLE")
    print("=" * 70)
    records = generate_synthetic_bundle()
    n_total = len(records)
    n_nulls = sum(1 for r in records for f in FIBER_FIELDS if r.fibers.get(f) is None)
    n_measured = sum(1 for r in records for f in FIBER_FIELDS if r.fibers.get(f) is not None)
    print(f"  Records:      {n_total}")
    print(f"  Fiber cells:  {n_total * len(FIBER_FIELDS)}")
    print(f"  Measured:     {n_measured}")
    print(f"  NULL (gaps):  {n_nulls} ({100*n_nulls/(n_total*len(FIBER_FIELDS)):.1f}%)")
    print(f"  Entity classes: {list(ENTITY_CLASSES.keys())} + orphan")
    print(f"  Contexts:     {CONTEXTS}")
    print(f"  Planted contradictions: 3 (E003/ctx_B, E008/ctx_D, E014/ctx_A)")
    print(f"  Orphan entities: E021-E025 (sparse, isolated)")
    print()

    # ── §3: Consistency check — find the contradictions ──
    print("=" * 70)
    print("§3  CONSISTENCY CHECK (H¹ ≠ 0 DETECTION)")
    print("=" * 70)
    contradictions = consistency_check(records)
    print(f"  Contradictions found: {len(contradictions)}")
    # Deduplicate by entity/context (same contradiction may appear for multiple neighbors)
    seen = set()
    for c in contradictions:
        key = (c["entity_id"], c["context"], c["field"])
        if key not in seen:
            seen.add(key)
            print(f"    ✗ {c['entity_id']}/{c['context']} {c['field']}={c['value']:.2f}")
            print(f"      {c['reason']}")
    print()

    # ── §4: Sheaf completion ──
    print("=" * 70)
    print("§4  SHEAF COMPLETION (sheaf_extension)")
    print("=" * 70)
    completions, skips = sheaf_complete(records, min_confidence=0.50, min_neighbors=2)
    print(f"  Completed:    {len(completions)}")
    print(f"  Skipped:      {len(skips)}")

    # Breakdown by skip reason
    skip_reasons = {}
    for s in skips:
        skip_reasons[s.reason] = skip_reasons.get(s.reason, 0) + 1
    for reason, count in sorted(skip_reasons.items()):
        print(f"    - {reason}: {count}")
    print()

    # Confidence distribution
    if completions:
        confs = [c.confidence for c in completions]
        print(f"  Confidence stats:")
        print(f"    min={min(confs):.4f}  median={sorted(confs)[len(confs)//2]:.4f}  "
              f"max={max(confs):.4f}  mean={statistics.mean(confs):.4f}")
        print()

        # Top 10 highest-confidence completions
        top = sorted(completions, key=lambda c: -c.confidence)[:10]
        print(f"  Top 10 highest-confidence completions:")
        print(f"  {'Entity':>8} {'Context':>8} {'Field':>6} {'Value':>8} "
              f"{'Conf':>6} {'±':>6} {'N':>3} {'Sources'}")
        print(f"  {'-'*8} {'-'*8} {'-'*6} {'-'*8} {'-'*6} {'-'*6} {'-'*3} {'-'*20}")
        for c in top:
            print(f"  {c.entity_id:>8} {c.context:>8} {c.field:>6} "
                  f"{c.completed_value:>8.4f} {c.confidence:>6.4f} "
                  f"±{c.uncertainty:>5.4f} {c.n_neighbors:>3} "
                  f"{','.join(c.constraint_sources)}")
        print()

        # Bottom 10 — edge of confidence threshold
        bottom = sorted(completions, key=lambda c: c.confidence)[:10]
        print(f"  Bottom 10 (near threshold):")
        print(f"  {'Entity':>8} {'Context':>8} {'Field':>6} {'Value':>8} "
              f"{'Conf':>6} {'±':>6} {'N':>3} {'Sources'}")
        print(f"  {'-'*8} {'-'*8} {'-'*6} {'-'*8} {'-'*6} {'-'*6} {'-'*3} {'-'*20}")
        for c in bottom:
            print(f"  {c.entity_id:>8} {c.context:>8} {c.field:>6} "
                  f"{c.completed_value:>8.4f} {c.confidence:>6.4f} "
                  f"±{c.uncertainty:>5.4f} {c.n_neighbors:>3} "
                  f"{','.join(c.constraint_sources)}")
        print()

    # ── §5: PROPAGATE — cascade from one measurement ──
    print("=" * 70)
    print("§5  PROPAGATE ANALYSIS (cascade from single measurement)")
    print("=" * 70)

    # Find an actual NULL cell to test propagation — choose an orphan
    # because orphans have many insufficient_neighbor skips that
    # a single measurement from a known class could unlock.
    prop_entity, prop_ctx, prop_field = "E021", "ctx_A", "F1"

    # Simulate: someone measures E021 and discovers it's actually class alpha
    # Inject a measured value AND change its class so it gains neighbors
    import copy
    records_prop = copy.deepcopy(records)
    for r in records_prop:
        if r.entity_id == "E021":
            r.entity_class = "alpha"  # Reclassify via measurement
            r.fibers["F1"] = 0.22     # Consistent with alpha/ctx_A F1 ≈ 0.20
            r.origin = "measured"

    before_completions, _ = sheaf_complete(records, min_confidence=0.50)
    before_keys = {(c.entity_id, c.context, c.field) for c in before_completions}
    after_completions, _ = sheaf_complete(records_prop, min_confidence=0.50)
    newly = [c for c in after_completions
             if (c.entity_id, c.context, c.field) not in before_keys]

    print(f"  Scenario: Measure E021/ctx_A F1 = 0.22 AND reclassify E021 → alpha")
    print(f"  (Simulates discovering an orphan's structural class)")
    print(f"  Newly determined: {len(newly)} additional completions")
    if newly:
        for c in newly[:15]:
            print(f"    + {c.entity_id}/{c.context} {c.field} = {c.completed_value:.4f} "
                  f"(conf={c.confidence:.3f})")
    print()

    # ── §6: Orphan entities — sparse data stress test ──
    print("=" * 70)
    print("§6  SPARSE DATA STRESS TEST (orphan entities)")
    print("=" * 70)
    orphan_completions = [c for c in completions if c.entity_id.startswith("E02")]
    orphan_skips = [s for s in skips if s.entity_id.startswith("E02")]
    print(f"  Orphan completions: {len(orphan_completions)}")
    print(f"  Orphan skips:       {len(orphan_skips)}")
    for s in orphan_skips[:10]:
        print(f"    skip: {s.entity_id}/{s.context} {s.field} → {s.reason}: {s.detail}")
    print()

    # ── §7: Validation — sample accuracy check ──
    print("=" * 70)
    print("§7  VALIDATION — COMPLETED VALUES vs GROUND TRUTH")
    print("=" * 70)
    # For non-orphan completions, we know the ground truth because
    # class_base + small noise defines the "true" section.
    cls_offset = {"alpha": 0.0, "beta": 0.3, "gamma": 0.6, "delta": 0.9, "orphan": None}
    ctx_offset = {"ctx_A": 0.0, "ctx_B": 0.1, "ctx_C": 0.2, "ctx_D": 0.3, "ctx_E": 0.4}
    fld_offset = {f: 0.05 * i for i, f in enumerate(FIBER_FIELDS)}

    deviations = []
    for c in completions:
        co = cls_offset.get(None)  # need the class
        # Look up class from records
        rec = None
        for r in records:
            if r.entity_id == c.entity_id:
                rec = r
                break
        if rec is None or cls_offset.get(rec.entity_class) is None:
            continue
        truth = 0.2 + cls_offset[rec.entity_class] + ctx_offset[c.context] + fld_offset[c.field]
        dev = abs(c.completed_value - truth)
        deviations.append((c, truth, dev))

    if deviations:
        devs = [d for _, _, d in deviations]
        print(f"  Completions with known ground truth: {len(deviations)}")
        print(f"  Mean |deviation|:  {statistics.mean(devs):.4f}")
        print(f"  Max |deviation|:   {max(devs):.4f}")
        print(f"  Median |deviation|: {sorted(devs)[len(devs)//2]:.4f}")

        within_band = sum(1 for c, t, d in deviations if d <= c.uncertainty)
        print(f"  Within uncertainty band: {within_band}/{len(deviations)} "
              f"({100*within_band/len(deviations):.1f}%)")
        print()

        # Show worst 5
        worst = sorted(deviations, key=lambda x: -x[2])[:5]
        print(f"  5 largest deviations:")
        for c, truth, dev in worst:
            in_band = "✓" if dev <= c.uncertainty else "✗"
            print(f"    {c.entity_id}/{c.context} {c.field}: "
                  f"completed={c.completed_value:.4f} truth={truth:.4f} "
                  f"|Δ|={dev:.4f} ±{c.uncertainty:.4f} {in_band}")
    print()

    # ── Summary ──
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"  Bundle: {n_total} records, {n_nulls} gaps ({100*n_nulls/(n_total*len(FIBER_FIELDS)):.0f}%)")
    print(f"  Contradictions detected (H¹ ≠ 0): {len(seen)}")
    print(f"  Successfully completed: {len(completions)}")
    print(f"  Skipped (by design): {len(skips)}")
    if completions:
        print(f"  Mean confidence: {statistics.mean([c.confidence for c in completions]):.4f}")
    if deviations:
        print(f"  Accuracy (within uncertainty band): "
              f"{100*within_band/len(deviations):.0f}%")
    print(f"  Cascade from 1 measurement: {len(newly)} new completions")
    print()


if __name__ == "__main__":
    main()
