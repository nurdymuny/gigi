//! `route_forecast_or_imagine` — pick between FORECAST and IMAGINE
//! given the local density signal at the query seed.
//!
//! Per Marcella round-3 feedback #3: the FORECAST vs IMAGINE routing
//! rule needs a computable θ. Without it, callers either always
//! IMAGINE (wasting work where the density gradient is meaningful) or
//! always FORECAST (missing the IMAGINE path in low-density regions).
//!
//! ## Rule
//!
//! - `query_grounding_normalized > THETA_DENSITY` → **FORECAST**. The
//!   density signal is meaningful; the density gradient predicts more
//!   accurately than parallel transport on the metric tensor.
//! - `query_grounding_normalized ≤ THETA_DENSITY` → **IMAGINE**. The
//!   density is too thin for a gradient; rely on the metric tensor
//!   and the substrate's geometric structure (geodesic +
//!   parallel-transport).
//!
//! ## Why θ = 0.5
//!
//! Anchored to Gate J — the refusal boundary in
//! `SUDOKU_PRIMITIVE_SPEC.md` where confidence-with-explain switches
//! between density-driven and structure-driven explanation. 0.5 is the
//! point at which the density signal stops dominating the metric
//! signal in the substrate's empirical calibration; lower than that,
//! the metric wins; higher, the density wins. The same threshold that
//! gates Marcella's confidence-with-explain pipeline gates the
//! FORECAST/IMAGINE choice — by design, so the two systems agree on
//! "density meaningful" vs "geometry meaningful."

use serde::{Deserialize, Serialize};

/// The FORECAST/IMAGINE routing threshold on query grounding density.
///
/// Anchored to Gate J's value in `SUDOKU_PRIMITIVE_SPEC.md` — the
/// substrate-wide boundary between "density signal meaningful" and
/// "metric signal meaningful." See module-level doc.
pub const THETA_DENSITY: f64 = 0.5;

/// The routing decision for a given query seed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDecision {
    /// Density signal is meaningful (above θ); FORECAST will produce
    /// better predictions than IMAGINE because the density gradient
    /// is informative.
    Forecast,
    /// Density signal is too thin (at or below θ); IMAGINE is correct
    /// because the metric tensor (and geodesic + parallel transport)
    /// is what predicts well in this regime.
    Imagine,
}

impl RoutingDecision {
    /// A short, audit-friendly string. Stable across versions.
    pub fn as_str(self) -> &'static str {
        match self {
            RoutingDecision::Forecast => "forecast",
            RoutingDecision::Imagine => "imagine",
        }
    }
}

/// Route the query to FORECAST or IMAGINE based on the seed's
/// normalized grounding density. The input is the same
/// `query_grounding_normalized` field that Gate J consumes.
///
/// `NaN` is treated as "no density signal" → IMAGINE (conservative —
/// IMAGINE refuses on high curvature, FORECAST does not).
#[inline]
pub fn route_forecast_or_imagine(query_grounding_normalized: f64) -> RoutingDecision {
    if query_grounding_normalized.is_nan() {
        return RoutingDecision::Imagine;
    }
    if query_grounding_normalized > THETA_DENSITY {
        RoutingDecision::Forecast
    } else {
        RoutingDecision::Imagine
    }
}

/// Advisory string surfaced in the `imagine_coherence` HTTP response
/// when a caller invokes IMAGINE but the density signal would have
/// routed to FORECAST. The endpoint still computes the trajectory —
/// this is just a signal that the caller may have mis-routed.
///
/// Per Marcella round-3 feedback #3: the endpoint should not silently
/// run IMAGINE when FORECAST is the better choice; it should surface
/// the routing advisory so the caller can adjust upstream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingAdvisory {
    /// The density used to make the routing decision.
    pub query_grounding_normalized: f64,
    /// The θ threshold against which it was compared.
    pub theta_density: f64,
    /// The recommended verb for this density.
    pub recommended: RoutingDecision,
    /// The verb actually invoked by the caller.
    pub invoked: RoutingDecision,
    /// True iff `recommended != invoked` — the caller mis-routed.
    pub mismatch: bool,
}

impl RoutingAdvisory {
    /// Build an advisory for an `imagine_coherence` invocation given
    /// the seed density. `invoked = Imagine` because the endpoint is
    /// the IMAGINE one; the recommended verb depends on density.
    pub fn for_imagine_invocation(query_grounding_normalized: f64) -> Self {
        let recommended = route_forecast_or_imagine(query_grounding_normalized);
        Self {
            query_grounding_normalized,
            theta_density: THETA_DENSITY,
            recommended,
            invoked: RoutingDecision::Imagine,
            mismatch: recommended != RoutingDecision::Imagine,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theta_density_is_anchored_to_gate_j() {
        // Per spec — if this drifts, Gate J and the FORECAST/IMAGINE
        // router will disagree on "density meaningful" and the
        // confidence pipeline + routing pipeline will diverge.
        assert_eq!(THETA_DENSITY, 0.5);
    }

    #[test]
    fn high_density_routes_to_forecast() {
        for d in [0.51, 0.7, 0.99, 1.0] {
            assert_eq!(route_forecast_or_imagine(d), RoutingDecision::Forecast,
                       "density {} should route to FORECAST", d);
        }
    }

    #[test]
    fn low_density_routes_to_imagine() {
        for d in [0.0, 0.1, 0.49, 0.5] {
            assert_eq!(route_forecast_or_imagine(d), RoutingDecision::Imagine,
                       "density {} should route to IMAGINE", d);
        }
    }

    #[test]
    fn boundary_at_theta_routes_to_imagine() {
        // ≤ θ is IMAGINE — boundary is inclusive on the IMAGINE side.
        // This is conservative: IMAGINE has the curvature ceiling
        // refusal that FORECAST does not, so the safer side gets the
        // boundary.
        assert_eq!(route_forecast_or_imagine(THETA_DENSITY), RoutingDecision::Imagine);
    }

    #[test]
    fn nan_density_routes_to_imagine() {
        // NaN = no density signal. Conservative routing chooses
        // IMAGINE because IMAGINE has the safety envelope; FORECAST
        // would silently produce a value on no signal.
        assert_eq!(route_forecast_or_imagine(f64::NAN), RoutingDecision::Imagine);
    }

    #[test]
    fn advisory_for_correct_imagine_invocation_has_no_mismatch() {
        let a = RoutingAdvisory::for_imagine_invocation(0.2);
        assert_eq!(a.recommended, RoutingDecision::Imagine);
        assert_eq!(a.invoked, RoutingDecision::Imagine);
        assert!(!a.mismatch);
    }

    #[test]
    fn advisory_for_misrouted_imagine_flags_mismatch() {
        // High-density caller invoked IMAGINE; should have invoked
        // FORECAST. Endpoint still runs but advisory surfaces the
        // mis-routing so the caller can adjust upstream.
        let a = RoutingAdvisory::for_imagine_invocation(0.8);
        assert_eq!(a.recommended, RoutingDecision::Forecast);
        assert_eq!(a.invoked, RoutingDecision::Imagine);
        assert!(a.mismatch);
    }

    #[test]
    fn routing_decision_as_str_is_stable() {
        // These strings ship in audit logs and HTTP responses. If they
        // drift, downstream parsers break.
        assert_eq!(RoutingDecision::Forecast.as_str(), "forecast");
        assert_eq!(RoutingDecision::Imagine.as_str(), "imagine");
    }

    #[test]
    fn routing_advisory_serde_round_trips() {
        let a = RoutingAdvisory::for_imagine_invocation(0.7);
        let json = serde_json::to_string(&a).unwrap();
        let back: RoutingAdvisory = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
