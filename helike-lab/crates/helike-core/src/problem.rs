//! A serialisable description of one sector problem, so that any group, metric, field and representation can be run
//! from a JSON file (the CLI's `solve`), and so that receipts record exactly what was computed.

use crate::charge::{charge_value, ChargeSplit, CircleGenerator};
use crate::lie::LieAlgebra;
use crate::linalg;
use crate::rep::{Rep, RepSpec};
use crate::sector::{Field, Geometry, Sector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlgebraSpec {
    /// [e_a, e_b] = 2 eps_abc e_c (frame of S^3 = SU(2)).
    Su2Frame,
    /// [tau_a, tau_b] = eps_abc tau_c.
    So3Cross,
    /// su(N), X = -(i/2) lambda (generalised Gell-Mann order).
    SuN { n: usize },
    /// Explicit structure constants, row-major [a][b][c].
    StructureConstants { dim: usize, c: Vec<f64> },
}

impl AlgebraSpec {
    pub fn build(&self) -> LieAlgebra {
        match self {
            AlgebraSpec::Su2Frame => LieAlgebra::su2_frame(),
            AlgebraSpec::So3Cross => LieAlgebra::so3_cross(),
            AlgebraSpec::SuN { n } => LieAlgebra::su_n(*n),
            AlgebraSpec::StructureConstants { dim, c } => LieAlgebra::new("explicit", *dim, c.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldSpec {
    /// M_i given directly, one gauge vector per orthonormal frame direction.
    Explicit { m: Vec<Vec<f64>> },
    /// A = s theta with the gauge algebra equal to the frame algebra: M_a = s unit_a / sqrt(g_a).
    MaurerCartan { s: f64 },
    /// The SU(2) circle-symmetric family member M_0 = x tau_0, M_1 = y tau_1, M_2 = y tau_2.
    Su2Family { x: f64, y: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CircleSpec {
    pub frame_gen: Vec<f64>,
    pub gauge_gen: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Problem {
    pub frame: AlgebraSpec,
    pub metric: Vec<f64>,
    pub gauge: AlgebraSpec,
    pub field: FieldSpec,
    pub rep: RepSpec,
    /// Optional circle symmetry to split the sector by.
    #[serde(default)]
    pub circle: Option<CircleSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockReport {
    pub charge: f64,
    pub size: usize,
    pub lowest_h_plus_ggstar: f64,
    pub lowest_slice: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub dims: (usize, usize, usize),
    pub sector_dim: usize,
    pub energy: f64,
    pub checks: Checks,
    pub spectrum_h: Vec<f64>,
    pub spectrum_slice: Vec<f64>,
    pub slice_rank_of_g: usize,
    pub blocks: Option<Vec<BlockReport>>,
    pub circle_signs: Option<(f64, f64)>,
    pub circle_residuals: Option<(f64, f64)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checks {
    pub frame_jacobi: f64,
    pub gauge_jacobi: f64,
    pub gauge_invariance: f64,
    pub rep_homomorphism: f64,
    pub h_hermitian: f64,
}

impl Problem {
    pub fn geometry(&self) -> Geometry {
        Geometry::new(self.frame.build(), self.metric.clone())
    }

    pub fn field(&self, geo: &Geometry) -> Field {
        let gauge = self.gauge.build();
        let m = match &self.field {
            FieldSpec::Explicit { m } => m.clone(),
            FieldSpec::MaurerCartan { s } => {
                assert_eq!(gauge.dim, geo.n(), "Maurer-Cartan needs gauge algebra = frame algebra");
                (0..geo.n()).map(|a| (0..gauge.dim).map(|b| if a == b { s / geo.metric[a].sqrt() } else { 0.0 }).collect()).collect()
            }
            FieldSpec::Su2Family { x, y } => vec![vec![*x, 0.0, 0.0], vec![0.0, *y, 0.0], vec![0.0, 0.0, *y]],
        };
        Field::new(gauge, m)
    }

    pub fn solve(&self) -> Report {
        let geo = self.geometry();
        let field = self.field(&geo);
        let rep = Rep::from_spec(&self.rep, geo.n());
        let sec = Sector::new(&geo, &field, &rep);
        let hg = sec.hessian_and_gauge();
        let (k, rank) = linalg::cokernel(&hg.g, 1e-10);
        let spectrum_slice = linalg::compressed_eigvalsh(&hg.h, &k);
        let checks = Checks {
            frame_jacobi: geo.frame.jacobi_error(),
            gauge_jacobi: field.gauge.jacobi_error(),
            gauge_invariance: field.gauge.invariance_error(),
            rep_homomorphism: rep.homomorphism_error(&geo.frame),
            h_hermitian: 0.0,
        };
        let (blocks, signs, resid) = match &self.circle {
            None => (None, None, None),
            Some(cs) => {
                let split = ChargeSplit::new(&sec, &hg, &CircleGenerator { frame_gen: cs.frame_gen.clone(), gauge_gen: cs.gauge_gen.clone() });
                let a = hg.h_plus_ggstar();
                let mins = split.block_minima(&a);
                let slice = split.slice_block_spectra(&hg);
                let blocks = split
                    .field_bases
                    .iter()
                    .map(|(q, b)| BlockReport {
                        charge: charge_value(*q),
                        size: b.ncols(),
                        lowest_h_plus_ggstar: mins[q],
                        lowest_slice: slice.get(q).and_then(|v| v.first().copied()),
                    })
                    .collect();
                (Some(blocks), Some(split.signs), Some((split.commutator_residual, split.intertwiner_residual)))
            }
        };
        Report {
            dims: sec.dims(),
            sector_dim: hg.dim(),
            energy: field.energy(&geo),
            checks,
            spectrum_h: hg.spectrum(),
            spectrum_slice,
            slice_rank_of_g: rank,
            blocks,
            circle_signs: signs,
            circle_residuals: resid,
        }
    }
}
