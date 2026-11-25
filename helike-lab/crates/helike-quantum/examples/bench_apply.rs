//! Time one block application of the cone operator: cargo run --release -p helike-quantum --example bench_apply -- MQ EXTRA
use helike_quantum::model::Background;
use helike_quantum::operator::{ConeOperator, Mutation, Part};
use helike_quantum::lanczos::seeded;
fn main() {
    let a: Vec<usize> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let (mq, extra) = (a.first().copied().unwrap_or(6), a.get(1).copied().unwrap_or(4));
    let bg = Background::maurer_cartan(1.15, 3.0, 0.004f64.sqrt()).unwrap();
    let op = ConeOperator::new(bg, mq, extra, Mutation::None).unwrap();
    let vs: Vec<Vec<f64>> = (0..4).map(|k| seeded(op.dim(), k)).collect();
    for part in [Part::Mass, Part::Full] {
        let t = std::time::Instant::now();
        let r = op.apply(&vs, part);
        println!("Mq={mq} Q={} N={} {part:?}: {:.3}s (checksum {:.6e})", op.grid.q, op.dim(), t.elapsed().as_secs_f64(), r[0].iter().sum::<f64>());
    }
}
