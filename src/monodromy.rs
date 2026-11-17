//! Bounded numerical SU(2) closure. See docs/MONODROMY_GROUP_ORDER_SPEC.md.
//! A verified finite table is not a certificate of exact f64 subgroup finiteness.
use crate::gauge::{group_element::GroupElement, registry, Group};
use crate::lattice::EdgeOrientation;
use crate::parser::ExecResult;
use crate::types::{Record, Value};
use std::collections::BTreeMap;

const UNIT_TOL: f64 = 1e-10;

fn coordinates(q: GroupElement) -> [f64; 4] {
    match q {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        _ => unreachable!("group checked before closure"),
    }
}

fn distance(a: GroupElement, b: GroupElement) -> f64 {
    coordinates(a)
        .iter()
        .zip(coordinates(b))
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn normalized(q: GroupElement) -> Result<(GroupElement, f64), String> {
    let a = coordinates(q);
    let norm = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !norm.is_finite() || (norm - 1.).abs() > UNIT_TOL {
        return Err(format!("MONODROMY_GROUP_ORDER: generator must be finite and unit-norm within {UNIT_TOL}; measured norm={norm}. Supply calibrated SU(2) quaternions."));
    }
    Ok((
        GroupElement::SU2 {
            q0: a[0] / norm,
            q1: a[1] / norm,
            q2: a[2] / norm,
            q3: a[3] / norm,
        },
        (norm - 1.).abs(),
    ))
}

// No rounded hash or antipodal identification: both lose group information.
fn locate(
    q: GroupElement,
    elements: &[GroupElement],
    tol: f64,
) -> Result<Option<(usize, f64)>, String> {
    if coordinates(q).iter().any(|x| !x.is_finite()) {
        return Err("nonfinite product; inspect generators and numerical conditioning".into());
    }
    let mut matched = None;
    let mut nearby = 0;
    for (i, e) in elements.iter().enumerate() {
        let d = distance(q, *e);
        if d <= 8. * tol {
            nearby += 1;
        }
        if d <= tol {
            matched = Some((i, d));
        }
    }
    if nearby > 1 || (nearby == 1 && matched.is_none()) {
        return Err("ambiguous tolerance neighborhood; supply better-resolved generators or a justified smaller TOLERANCE".into());
    }
    Ok(matched)
}

fn verify_table(table: &[Vec<usize>], checks: &mut usize) -> Result<Vec<usize>, String> {
    let n = table.len();
    if n == 0
        || table
            .iter()
            .any(|r| r.len() != n || r.iter().any(|&x| x >= n))
    {
        return Err("invalid multiplication table dimensions or indices".into());
    }
    let mut inverses = Vec::with_capacity(n);
    for a in 0..n {
        if table[0][a] != a || table[a][0] != a {
            return Err("table identity check failed".into());
        }
        inverses.push(
            (0..n)
                .find(|&b| table[a][b] == 0 && table[b][a] == 0)
                .ok_or_else(|| "table two-sided inverse check failed".to_string())?,
        );
    }
    for a in 0..n {
        for b in 0..n {
            for c in 0..n {
                *checks += 1;
                if table[table[a][b]][c] != table[a][table[b][c]] {
                    return Err(
                        "table associativity check failed; generators are not numerically resolved"
                            .into(),
                    );
                }
            }
        }
    }
    Ok(inverses)
}

fn derived_order(table: &[Vec<usize>], inverses: &[usize]) -> usize {
    let n = table.len();
    let mut is_generator = vec![false; n];
    for a in 0..n {
        for b in 0..n {
            let c = table[table[table[a][b]][inverses[a]]][inverses[b]];
            is_generator[c] = true;
        }
    }
    let gens: Vec<_> = (0..n).filter(|&i| is_generator[i]).collect();
    let mut seen = vec![false; n];
    seen[0] = true;
    let mut queue = vec![0];
    let mut head = 0;
    while head < queue.len() {
        for &b in &gens {
            let c = table[queue[head]][b];
            if !seen[c] {
                seen[c] = true;
                queue.push(c);
            }
        }
        head += 1;
    }
    queue.len()
}

struct Closure {
    elements: Vec<GroupElement>,
    products: usize,
    checks: usize,
    residual: f64,
}

impl Closure {
    fn product(&mut self, a: GroupElement, b: GroupElement) -> GroupElement {
        self.products += 1;
        a.compose(&b)
    }
    fn compute(
        &mut self,
        gens: &[GroupElement],
        cap: usize,
        tol: f64,
    ) -> Result<(Vec<Vec<usize>>, Vec<usize>), String> {
        let mut steps = gens.to_vec();
        steps.extend(gens.iter().map(GroupElement::inverse));
        let mut head = 0;
        while head < self.elements.len() {
            for &g in &steps {
                let q = self.product(self.elements[head], g);
                match locate(q, &self.elements, tol)? {
                    Some((_, d)) => self.residual = self.residual.max(d),
                    None => {
                        if self.elements.len() >= cap {
                            return Err(format!("CAP {cap} exhausted before closure; increase CAP within 256 or use a different representation. This does not establish an infinite group."));
                        }
                        self.elements.push(q);
                    }
                }
            }
            head += 1;
        }
        let n = self.elements.len();
        let mut table = vec![vec![0; n]; n];
        for a in 0..n {
            for b in 0..n {
                let q = self.product(self.elements[a], self.elements[b]);
                let (i, d) = locate(q, &self.elements, tol)?.ok_or_else(|| {
                    "full multiplication table did not close; supply better-resolved generators"
                        .to_string()
                })?;
                self.residual = self.residual.max(d);
                table[a][b] = i;
            }
        }
        let inverses = verify_table(&table, &mut self.checks)?;
        Ok((table, inverses))
    }
}

pub fn execute_monodromy_group_order(
    field: &str,
    edges: &[usize],
    commutator: bool,
    cap: usize,
    tolerance: f64,
) -> Result<ExecResult, String> {
    if edges.is_empty()
        || edges.len() > 32
        || !(1..=256).contains(&cap)
        || !tolerance.is_finite()
        || !(1e-12..=1e-6).contains(&tolerance)
    {
        return Err("MONODROMY_GROUP_ORDER: require 1..32 generator edges, CAP 1..256, and finite TOLERANCE in [1e-12,1e-6]".into());
    }
    let handle=registry::get(field).ok_or_else(|| format!(
        "MONODROMY_GROUP_ORDER: field '{field}' not declared; use GAUGE_FIELD ... INIT FROM BUNDLE ... GROUP SU(2) first"))?;
    if handle.group() != Group::SU2 {
        return Err(format!(
            "MONODROMY_GROUP_ORDER requires GROUP SU(2); got {}. Supply SU(2) generator elements.",
            handle.group().label()
        ));
    }
    let buffer = handle.as_dense_buffer();
    if buffer.group != Group::SU2
        || buffer.repr_dim != 4
        || buffer.n_edges.checked_mul(4) != Some(buffer.data.len())
    {
        return Err("MONODROMY_GROUP_ORDER: malformed SU(2) field buffer; recreate the field from validated generator records".into());
    }
    let mut generators = Vec::with_capacity(edges.len());
    let mut norm_error = 0f64;
    for &edge in edges {
        if edge >= buffer.n_edges {
            return Err(format!(
                "MONODROMY_GROUP_ORDER: generator edge {edge} out of range; field has {} edges",
                buffer.n_edges
            ));
        }
        let (q, err) = normalized(handle.edge_element(edge, EdgeOrientation::Forward))?;
        norm_error = norm_error.max(err);
        generators.push(q);
    }
    let mut closure = Closure {
        elements: vec![GroupElement::su2_identity()],
        products: 0,
        checks: 0,
        residual: 0.,
    };
    let result = closure.compute(&generators, cap, tolerance);
    let mut row = Record::new();
    for key in [
        "group_order",
        "commutator_subgroup_order",
        "is_perfect",
        "abelianization_order",
        "reason",
    ] {
        row.insert(key.into(), Value::Null);
    }
    row.insert("element_orders".into(), Value::Vector(vec![]));
    row.insert("element_order_counts".into(), Value::Vector(vec![]));
    row.insert("group_used".into(), Value::Text("SU(2)".into()));
    row.insert(
        "scope".into(),
        Value::Text("supplied_generator_subgroup".into()),
    );
    row.insert(
        "numerical_policy".into(),
        Value::Text("su2_closure_v1".into()),
    );
    row.insert("exact_subgroup_certified".into(), Value::Bool(false));
    row.insert("table_verified".into(), Value::Bool(result.is_ok()));
    row.insert("cap".into(), Value::Integer(cap as i64));
    row.insert("tolerance".into(), Value::Float(tolerance));
    row.insert("generator_count".into(), Value::Integer(edges.len() as i64));
    row.insert(
        "generator_edges".into(),
        Value::Vector(edges.iter().map(|&x| x as f64).collect()),
    );
    row.insert(
        "n_elements_observed".into(),
        Value::Integer(closure.elements.len() as i64),
    );
    row.insert(
        "products_evaluated".into(),
        Value::Integer(closure.products as i64),
    );
    row.insert(
        "associativity_checks".into(),
        Value::Integer(closure.checks as i64),
    );
    row.insert("max_input_norm_error".into(), Value::Float(norm_error));
    row.insert(
        "max_closure_residual".into(),
        Value::Float(closure.residual),
    );
    let mut separation = f64::INFINITY;
    for (i, a) in closure.elements.iter().enumerate() {
        for &b in &closure.elements[..i] {
            separation = separation.min(distance(*a, b));
        }
    }
    row.insert(
        "min_element_separation".into(),
        if separation.is_finite() {
            Value::Float(separation)
        } else {
            Value::Null
        },
    );
    match result {
        Err(reason) => {
            row.insert("status".into(), Value::Text("unresolved".into()));
            row.insert("reason".into(), Value::Text(reason));
        }
        Ok((table, inverses)) => {
            let n = table.len();
            let mut histogram = BTreeMap::<usize, usize>::new();
            for a in 0..n {
                let mut p = 0;
                for order in 1..=n {
                    p = table[p][a];
                    if p == 0 {
                        *histogram.entry(order).or_default() += 1;
                        break;
                    }
                }
            }
            row.insert("status".into(), Value::Text("numerically_closed".into()));
            row.insert("group_order".into(), Value::Integer(n as i64));
            row.insert(
                "element_orders".into(),
                Value::Vector(histogram.keys().map(|&x| x as f64).collect()),
            );
            row.insert(
                "element_order_counts".into(),
                Value::Vector(histogram.values().map(|&x| x as f64).collect()),
            );
            if commutator {
                let d = derived_order(&table, &inverses);
                row.insert("commutator_subgroup_order".into(), Value::Integer(d as i64));
                row.insert("is_perfect".into(), Value::Bool(d == n));
                row.insert(
                    "abelianization_order".into(),
                    Value::Integer((n / d) as i64),
                );
            }
        }
    }
    Ok(ExecResult::Rows(vec![row]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(a: f64) -> GroupElement {
        GroupElement::SU2 {
            q0: a.cos(),
            q1: a.sin(),
            q2: 0.,
            q3: 0.,
        }
    }
    #[test]
    fn mg06_matcher_rejects_annulus_and_multiple_matches() {
        assert!(locate(q(2e-8), &[q(0.)], 1e-8).is_err());
        assert!(locate(q(0.), &[q(0.), q(5e-9)], 1e-8).is_err());
        assert_eq!(locate(q(0.), &[q(0.)], 1e-8).unwrap(), Some((0, 0.)));
        assert_eq!(locate(q(1.), &[q(0.)], 1e-8).unwrap(), None);
    }
    #[test]
    fn mg06_table_certifier_rejects_bad_evidence() {
        let mut checks = 0;
        assert!(verify_table(&[vec![0, 1], vec![1, 0]], &mut checks).is_ok());
        assert_eq!(checks, 8);
        assert!(verify_table(&[vec![1, 0], vec![0, 1]], &mut 0)
            .unwrap_err()
            .contains("identity"));
        assert!(verify_table(&[vec![0, 1], vec![1, 1]], &mut 0)
            .unwrap_err()
            .contains("inverse"));
        let nonassoc = vec![vec![0, 1, 2], vec![1, 0, 1], vec![2, 1, 0]];
        assert!(verify_table(&nonassoc, &mut 0)
            .unwrap_err()
            .contains("associativity"));
        assert!(verify_table(&[vec![7]], &mut 0).is_err());
    }
}
