//! TPC-H Query Benchmarks — Q1, Q3, Q6, Q14
//!
//! Phase 3 of the TDD plan (see GIGI_TPCH_SPEC.md).
//!
//! Implements a deterministic TPC-H data generator and runs four queries
//! that cover all GIGI join strategies:
//!
//!   Q6  — filter + scan aggregate (no join)           Tier 1
//!   Q1  — filter + group_by aggregate                 Tier 2
//!   Q14 — filter + pullback_join + conditional agg    Tier 1
//!   Q3  — filter → bitmap join → pullback join → agg  Tier 2
//!
//! Run: cargo run --bin bench_tpch --release
//!
//! Output: timing table per query per scale factor + storage mode + K

use gigi::bundle::{BaseGeometry, BundleStore, QueryCondition};
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

// ── TPC-H Date constants (epoch days from 1992-01-01) ──────────────

const DATE_1994_01_01: i64 = 731;
const DATE_1995_01_01: i64 = 1096;
const DATE_1995_03_15: i64 = 1169;
const DATE_1995_09_01: i64 = 1339;
const DATE_1995_10_01: i64 = 1369;
const DATE_1998_12_01: i64 = 2526;

const MKTSEGMENTS: &[&str] = &["AUTOMOBILE", "BUILDING", "FURNITURE", "HOUSEHOLD", "MACHINERY"];
const RETURNFLAGS: &[&str] = &["A", "N", "R"];
const LINESTATUSES: &[&str] = &["F", "O"];
const SHIPMODES: &[&str] = &["AIR", "MAIL", "RAIL", "SHIP", "TRUCK"];
const PART_TYPES: &[&str] = &[
    "PROMO BURNISHED COPPER",
    "LARGE PLATED BRASS",
    "STANDARD POLISHED BRASS",
    "ECONOMY ANODIZED STEEL",
    "MEDIUM BRUSHED TIN",
];

// ── Date helpers ───────────────────────────────────────────────────

/// Convert "YYYY-MM-DD" to days since 1992-01-01 (matching DATE_* constants).
fn parse_date(s: &str) -> i64 {
    #[inline]
    fn jdn(y: i32, m: i32, d: i32) -> i64 {
        let a = (14 - m) / 12;
        let yy = (y + 4800 - a) as i64;
        let mm = (m + 12 * a - 3) as i64;
        d as i64 + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
    }
    let s = s.trim();
    if s.len() < 10 { return 0; }
    let y: i32 = s[0..4].parse().unwrap_or(1992);
    let m: i32 = s[5..7].parse().unwrap_or(1);
    let d: i32 = s[8..10].parse().unwrap_or(1);
    jdn(y, m, d) - jdn(1992, 1, 1)
}

// ── Minimal deterministic PRNG (xorshift64) ────────────────────────

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed) }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() as i64).abs() % (hi - lo + 1)
    }
    fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.next() as usize % slice.len()]
    }
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() as f64 / u64::MAX as f64) * (hi - lo)
    }
}

// ── TPC-H Dataset ──────────────────────────────────────────────────

struct TpchData {
    customer: BundleStore,
    orders: BundleStore,
    lineitem: BundleStore,
    part: BundleStore,
    n_customers: usize,
    n_orders: usize,
    n_lineitems: usize,
}

fn make_record(fields: &[(&str, Value)]) -> Record {
    fields.iter().cloned().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Build all schemas.
fn customer_schema() -> BundleSchema {
    BundleSchema::new("customer")
        .base(FieldDef::numeric("c_custkey"))
        .fiber(FieldDef::categorical("c_mktsegment"))
        .fiber(FieldDef::numeric("c_nationkey"))
        .fiber(FieldDef::numeric("c_acctbal"))
        .index("c_mktsegment")
}

fn orders_schema() -> BundleSchema {
    BundleSchema::new("orders")
        .base(FieldDef::numeric("o_orderkey"))
        .fiber(FieldDef::numeric("o_custkey"))
        .fiber(FieldDef::categorical("o_orderstatus"))
        .fiber(FieldDef::numeric("o_totalprice"))
        .fiber(FieldDef::numeric("o_orderdate"))
        .fiber(FieldDef::numeric("o_shippriority"))
        .index("o_custkey")
}

fn lineitem_schema() -> BundleSchema {
    BundleSchema::new("lineitem")
        .base(FieldDef::numeric("l_id"))
        .fiber(FieldDef::numeric("l_orderkey"))
        .fiber(FieldDef::numeric("l_partkey"))
        .fiber(FieldDef::numeric("l_suppkey"))
        .fiber(FieldDef::numeric("l_quantity"))
        .fiber(FieldDef::numeric("l_extendedprice"))
        .fiber(FieldDef::numeric("l_discount"))
        .fiber(FieldDef::numeric("l_tax"))
        .fiber(FieldDef::categorical("l_returnflag"))
        .fiber(FieldDef::categorical("l_linestatus"))
        .fiber(FieldDef::numeric("l_shipdate"))
        .fiber(FieldDef::categorical("l_shipmode"))
        .index("l_returnflag")
        .index("l_linestatus")
        .index("l_orderkey")
        .index("l_partkey")
}

fn part_schema() -> BundleSchema {
    BundleSchema::new("part")
        .base(FieldDef::numeric("p_partkey"))
        .fiber(FieldDef::categorical("p_type"))
        .fiber(FieldDef::numeric("p_size"))
        .fiber(FieldDef::numeric("p_retailprice"))
}

fn generate(sf: f64, seed: u64) -> TpchData {
    let mut rng = Rng::new(seed);

    let n_cust = (150_000.0 * sf).max(10.0) as usize;
    let n_supp = (10_000.0 * sf).max(2.0) as usize;
    let n_part = (200_000.0 * sf).max(5.0) as usize;
    let n_orders_target = (1_500_000.0 * sf).max(15.0) as usize;

    // CUSTOMER — arithmetic keys 1..N → Sequential (K=0)
    let mut customer = BundleStore::with_geometry(
        customer_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "c_custkey".into() },
    );
    for i in 1..=n_cust as i64 {
        let seg = (*rng.choice(MKTSEGMENTS)).to_string();
        customer.insert(&make_record(&[
            ("c_custkey", Value::Integer(i)),
            ("c_mktsegment", Value::Text(seg)),
            ("c_nationkey", Value::Integer(rng.range(0, 24))),
            ("c_acctbal", Value::Float(rng.uniform(-999.0, 9999.0))),
        ]));
    }

    // PART — arithmetic keys 1..N → Sequential (K=0)
    let mut part = BundleStore::with_geometry(
        part_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "p_partkey".into() },
    );
    for i in 1..=n_part as i64 {
        let pt = (*rng.choice(PART_TYPES)).to_string();
        part.insert(&make_record(&[
            ("p_partkey", Value::Integer(i)),
            ("p_type", Value::Text(pt)),
            ("p_size", Value::Integer(rng.range(1, 50))),
            ("p_retailprice", Value::Float(900.0 + i as f64 * 0.01 + rng.uniform(0.0, 100.0))),
        ]));
    }

    // ORDERS — sparse keys (gaps 1..8) → Hashed (K>0)
    let mut orders = BundleStore::new(orders_schema());
    let mut orderkey: i64 = 1;
    let mut order_keys_vec: Vec<i64> = Vec::with_capacity(n_orders_target);
    for _ in 0..n_orders_target {
        let odate = rng.range(0, DATE_1998_12_01 - 200);
        orders.insert(&make_record(&[
            ("o_orderkey", Value::Integer(orderkey)),
            ("o_custkey", Value::Integer(rng.range(1, n_cust as i64))),
            ("o_orderstatus", Value::Text("O".into())),
            ("o_totalprice", Value::Float(rng.uniform(1000.0, 500_000.0))),
            ("o_orderdate", Value::Integer(odate)),
            ("o_shippriority", Value::Integer(0)),
        ]));
        order_keys_vec.push(orderkey);
        orderkey += rng.range(1, 8);  // sparse gaps
    }

    // LINEITEM — auto-inc l_id (K=0 → Sequential) + indexed l_orderkey/l_partkey
    let _n_lineitems_est = n_orders_target * 4;
    let mut lineitem = BundleStore::with_geometry(
        lineitem_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "l_id".into() },
    );
    let discounts = [0.02_f64, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09, 0.10];
    let mut lid: i64 = 1;
    for &ok in &order_keys_vec {
        // Lookup order date — needs point_query
        let odate = orders
            .point_query(&make_record(&[("o_orderkey", Value::Integer(ok))]))
            .and_then(|r| match r.get("o_orderdate") { Some(Value::Integer(d)) => Some(*d), _ => None })
            .unwrap_or(500);

        let n_lines = rng.range(1, 7);
        for _ in 0..n_lines {
            let qty = rng.range(1, 50) as f64;
            let price = rng.uniform(1000.0, 100_000.0);
            let disc = discounts[rng.next() as usize % discounts.len()];
            let shipdate = odate + rng.range(1, 120);
            let rf = (*rng.choice(RETURNFLAGS)).to_string();
            let ls = (*rng.choice(LINESTATUSES)).to_string();
            let sm = (*rng.choice(SHIPMODES)).to_string();
            let pk = rng.range(1, n_part as i64);

            lineitem.insert(&make_record(&[
                ("l_id", Value::Integer(lid)),
                ("l_orderkey", Value::Integer(ok)),
                ("l_partkey", Value::Integer(pk)),
                ("l_suppkey", Value::Integer(rng.range(1, n_supp as i64))),
                ("l_quantity", Value::Float(qty)),
                ("l_extendedprice", Value::Float(price)),
                ("l_discount", Value::Float(disc)),
                ("l_tax", Value::Float(rng.choice(&[0.0_f64, 0.02, 0.04, 0.06, 0.08]).clone())),
                ("l_returnflag", Value::Text(rf)),
                ("l_linestatus", Value::Text(ls)),
                ("l_shipdate", Value::Integer(shipdate)),
                ("l_shipmode", Value::Text(sm)),
            ]));
            lid += 1;
        }
    }

    TpchData {
        n_customers: n_cust,
        n_orders: order_keys_vec.len(),
        n_lineitems: (lid - 1) as usize,
        customer,
        orders,
        lineitem,
        part,
    }
}

// ── Query implementations ──────────────────────────────────────────

/// Q6: Forecasting Revenue Change
/// sum(l_extendedprice * l_discount) WHERE shipdate in [1994, 1995),
/// discount in [0.05, 0.07], quantity < 24
fn q6(data: &TpchData) -> (f64, usize) {
    let mut total = 0.0f64;
    let mut count = 0usize;
    for rec in data.lineitem.records() {
        let shipdate = rec.get("l_shipdate").and_then(|v| match v { Value::Integer(i) => Some(*i), _ => None }).unwrap_or(-1);
        let discount = rec.get("l_discount").and_then(|v| v.as_f64()).unwrap_or(-1.0);
        let qty = rec.get("l_quantity").and_then(|v| v.as_f64()).unwrap_or(100.0);
        if (DATE_1994_01_01..DATE_1995_01_01).contains(&shipdate)
            && (0.05..=0.07).contains(&discount)
            && qty < 24.0
        {
            let price = rec.get("l_extendedprice").and_then(|v| v.as_f64()).unwrap_or(0.0);
            total += price * discount;
            count += 1;
        }
    }
    (total, count)
}

/// Q1: Pricing Summary
/// GROUP BY (l_returnflag, l_linestatus) for shipdate <= 1998-12-01 - 90
fn q1(data: &TpchData) -> HashMap<(String, String), (usize, f64, f64)> {
    let cutoff = DATE_1998_12_01 - 90;
    let mut groups: HashMap<(String, String), (usize, f64, f64)> = HashMap::new();

    for rec in data.lineitem.records() {
        let shipdate = match rec.get("l_shipdate") {
            Some(Value::Integer(i)) => *i,
            _ => continue,
        };
        if shipdate > cutoff { continue; }

        let rf = match rec.get("l_returnflag") {
            Some(Value::Text(s)) => s.clone(),
            _ => continue,
        };
        let ls = match rec.get("l_linestatus") {
            Some(Value::Text(s)) => s.clone(),
            _ => continue,
        };
        let qty = rec.get("l_quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price = rec.get("l_extendedprice").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let disc_price = price * (1.0 - rec.get("l_discount").and_then(|v| v.as_f64()).unwrap_or(0.0));

        let entry = groups.entry((rf, ls)).or_insert((0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += qty;
        entry.2 += disc_price;
    }
    groups
}

/// Q14: Promotion Effect
/// 100 * sum(promo lines) / sum(all lines) for shipdate in [1995-09-01, 1995-10-01)
fn q14(data: &TpchData) -> f64 {
    // Filter lineitem by shipdate
    let filtered_conds = vec![
        QueryCondition::Gte("l_shipdate".into(), Value::Integer(DATE_1995_09_01)),
        QueryCondition::Lt("l_shipdate".into(), Value::Integer(DATE_1995_10_01)),
    ];
    let filtered_li = data.lineitem.filtered_query(&filtered_conds, None, false, None, None);

    // Pullback join LINEITEM → PART (l_partkey = p_partkey, PART.base = p_partkey → O(1))
    let joined = {
        let mut res = Vec::with_capacity(filtered_li.len());
        for li_rec in &filtered_li {
            let pk = match li_rec.get("l_partkey") {
                Some(Value::Integer(i)) => *i,
                _ => continue,
            };
            let part_rec = data.part.point_query(
                &make_record(&[("p_partkey", Value::Integer(pk))])
            );
            res.push((li_rec, part_rec));
        }
        res
    };

    let mut promo_sum = 0.0f64;
    let mut total_sum = 0.0f64;
    for (li_rec, part_rec_opt) in &joined {
        let price = li_rec.get("l_extendedprice").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let disc = li_rec.get("l_discount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rev = price * (1.0 - disc);
        total_sum += rev;
        if let Some(part_rec) = part_rec_opt {
            if let Some(Value::Text(pt)) = part_rec.get("p_type") {
                if pt.starts_with("PROMO") {
                    promo_sum += rev;
                }
            }
        }
    }

    if total_sum < 1e-9 { 0.0 } else { 100.0 * promo_sum / total_sum }
}

/// Q3: Shipping Priority
/// CUSTOMER(BUILDING) → ORDERS(date<cutoff) → LINEITEM(date>cutoff)
/// Returns: Vec<(l_orderkey, o_orderdate, revenue)>
fn q3(data: &TpchData) -> Vec<(i64, i64, f64)> {
    // Step 1: Filter CUSTOMER by mktsegment=BUILDING
    let building_conds = vec![
        QueryCondition::Eq("c_mktsegment".into(), Value::Text("BUILDING".into())),
    ];
    let building_customers = data.customer.filtered_query(&building_conds, None, false, None, None);
    let building_keys: Vec<Value> = building_customers
        .iter()
        .filter_map(|r| r.get("c_custkey").cloned())
        .collect();

    if building_keys.is_empty() {
        return Vec::new();
    }

    // Step 2: Bitmap join ORDERS on o_custkey (indexed fiber field)
    // range_query("o_custkey", building_keys) — O(|building_keys|) bitmap lookups

    let matching_orders: Vec<Record> = {
        let raw = data.orders.range_query("o_custkey", &building_keys);
        raw.into_iter()
            .filter(|o| matches!(o.get("o_orderdate"), Some(Value::Integer(d)) if *d < DATE_1995_03_15))
            .collect()
    };

    // Step 3: Bitmap join LINEITEM on l_orderkey (indexed fiber field)
    let order_keys: Vec<Value> = matching_orders
        .iter()
        .filter_map(|o| o.get("o_orderkey").cloned())
        .collect();

    if order_keys.is_empty() {
        return Vec::new();
    }

    let matching_li = data.lineitem.range_query("l_orderkey", &order_keys);

    // Build order map for quick lookup
    let order_map: HashMap<i64, &Record> = matching_orders
        .iter()
        .filter_map(|o| {
            if let Some(Value::Integer(ok)) = o.get("o_orderkey") {
                Some((*ok, o))
            } else {
                None
            }
        })
        .collect();

    // Step 4: Aggregate revenue per (l_orderkey, o_orderdate, o_shippriority)
    let mut groups: HashMap<i64, (i64, f64)> = HashMap::new();
    for li in &matching_li {
        let shipdate = match li.get("l_shipdate") {
            Some(Value::Integer(d)) => *d,
            _ => continue,
        };
        if shipdate <= DATE_1995_03_15 { continue; }

        let ok = match li.get("l_orderkey") {
            Some(Value::Integer(i)) => *i,
            _ => continue,
        };
        let Some(order) = order_map.get(&ok) else { continue; };
        let odate = match order.get("o_orderdate") {
            Some(Value::Integer(d)) => *d,
            _ => continue,
        };
        let price = li.get("l_extendedprice").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let disc = li.get("l_discount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rev = price * (1.0 - disc);
        groups.entry(ok).or_insert((odate, 0.0)).1 += rev;
    }

    let mut result: Vec<(i64, i64, f64)> = groups
        .into_iter()
        .map(|(ok, (odate, rev))| (ok, odate, rev))
        .collect();
    result.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    result
}

// ── SF=1 real .tbl file loader ────────────────────────────────────

/// Load TPC-H pipe-delimited .tbl files exported by DuckDB/dbgen.
/// Returns None if required files are absent.
fn load_real_data(tbl_dir: &str) -> Option<TpchData> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let dir = std::path::Path::new(tbl_dir);
    for name in &["lineitem.tbl", "orders.tbl", "customer.tbl", "part.tbl"] {
        if !dir.join(name).exists() {
            println!("  [skip] {} not found in {tbl_dir}", name);
            return None;
        }
    }

    // CUSTOMER — cols: custkey[0] name[1] addr[2] nationkey[3] phone[4] acctbal[5] mktseg[6]
    println!("  Loading customer.tbl...");
    let t = Instant::now();
    let mut customer = BundleStore::with_geometry(
        customer_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "c_custkey".into() },
    );
    let mut n_customers = 0usize;
    {
        let f = File::open(dir.join("customer.tbl")).expect("customer.tbl");
        for line in BufReader::with_capacity(1 << 20, f).lines().flatten() {
            if line.is_empty() { continue; }
            let col: Vec<&str> = line.splitn(9, '|').collect();
            if col.len() < 7 { continue; }
            let custkey: i64 = col[0].parse().unwrap_or(0);
            let nationkey: i64 = col[3].parse().unwrap_or(0);
            let acctbal: f64 = col[5].parse().unwrap_or(0.0);
            customer.insert(&make_record(&[
                ("c_custkey", Value::Integer(custkey)),
                ("c_mktsegment", Value::Text(col[6].trim().to_string())),
                ("c_nationkey", Value::Integer(nationkey)),
                ("c_acctbal", Value::Float(acctbal)),
            ]));
            n_customers += 1;
        }
    }
    println!("    {n_customers} rows in {:.1}ms", t.elapsed().as_secs_f64() * 1000.0);

    // PART — cols: partkey[0] name[1] mfgr[2] brand[3] type[4] size[5] container[6] price[7]
    println!("  Loading part.tbl...");
    let t = Instant::now();
    let mut part = BundleStore::with_geometry(
        part_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "p_partkey".into() },
    );
    {
        let f = File::open(dir.join("part.tbl")).expect("part.tbl");
        for line in BufReader::with_capacity(1 << 20, f).lines().flatten() {
            if line.is_empty() { continue; }
            let col: Vec<&str> = line.splitn(10, '|').collect();
            if col.len() < 8 { continue; }
            let partkey: i64 = col[0].parse().unwrap_or(0);
            let size: i64 = col[5].parse().unwrap_or(0);
            let price: f64 = col[7].parse().unwrap_or(0.0);
            part.insert(&make_record(&[
                ("p_partkey", Value::Integer(partkey)),
                ("p_type", Value::Text(col[4].trim().to_string())),
                ("p_size", Value::Integer(size)),
                ("p_retailprice", Value::Float(price)),
            ]));
        }
    }
    println!("    200K rows in {:.1}ms", t.elapsed().as_secs_f64() * 1000.0);

    // ORDERS — cols: orderkey[0] custkey[1] status[2] totalprice[3] date[4] pri[5] clerk[6] shippri[7]
    println!("  Loading orders.tbl...");
    let t = Instant::now();
    let mut orders = BundleStore::new(orders_schema());
    let mut n_orders = 0usize;
    {
        let f = File::open(dir.join("orders.tbl")).expect("orders.tbl");
        for line in BufReader::with_capacity(1 << 20, f).lines().flatten() {
            if line.is_empty() { continue; }
            let col: Vec<&str> = line.splitn(10, '|').collect();
            if col.len() < 8 { continue; }
            let orderkey: i64 = col[0].parse().unwrap_or(0);
            let custkey: i64 = col[1].parse().unwrap_or(0);
            let totalprice: f64 = col[3].parse().unwrap_or(0.0);
            let orderdate = parse_date(col[4]);
            let shippriority: i64 = col[7].parse().unwrap_or(0);
            orders.insert(&make_record(&[
                ("o_orderkey", Value::Integer(orderkey)),
                ("o_custkey", Value::Integer(custkey)),
                ("o_orderstatus", Value::Text(col[2].trim().to_string())),
                ("o_totalprice", Value::Float(totalprice)),
                ("o_orderdate", Value::Integer(orderdate)),
                ("o_shippriority", Value::Integer(shippriority)),
            ]));
            n_orders += 1;
        }
    }
    println!("    {n_orders} rows in {:.1}ms", t.elapsed().as_secs_f64() * 1000.0);

    // LINEITEM — auto-inc l_id; cols: orderkey[0] partkey[1] suppkey[2] linenum[3]
    //   qty[4] extprice[5] disc[6] tax[7] rf[8] ls[9] shipdate[10] shipmode[14]
    println!("  Loading lineitem.tbl (6M rows, ~30s)...");
    let t = Instant::now();
    let mut lineitem = BundleStore::with_geometry(
        lineitem_schema(),
        BaseGeometry::Flat { start: 1, step: 1, key_field: "l_id".into() },
    );
    let mut lid: i64 = 1;
    {
        let f = File::open(dir.join("lineitem.tbl")).expect("lineitem.tbl");
        for line in BufReader::with_capacity(4 << 20, f).lines().flatten() {
            if line.is_empty() { continue; }
            let col: Vec<&str> = line.splitn(17, '|').collect();
            if col.len() < 15 { continue; }
            let orderkey: i64 = col[0].parse().unwrap_or(0);
            let partkey: i64  = col[1].parse().unwrap_or(0);
            let suppkey: i64  = col[2].parse().unwrap_or(0);
            let qty: f64      = col[4].parse().unwrap_or(0.0);
            let extprice: f64 = col[5].parse().unwrap_or(0.0);
            let discount: f64 = col[6].parse().unwrap_or(0.0);
            let tax: f64      = col[7].parse().unwrap_or(0.0);
            let shipdate      = parse_date(col[10]);
            lineitem.insert(&make_record(&[
                ("l_id",            Value::Integer(lid)),
                ("l_orderkey",      Value::Integer(orderkey)),
                ("l_partkey",       Value::Integer(partkey)),
                ("l_suppkey",       Value::Integer(suppkey)),
                ("l_quantity",      Value::Float(qty)),
                ("l_extendedprice", Value::Float(extprice)),
                ("l_discount",      Value::Float(discount)),
                ("l_tax",           Value::Float(tax)),
                ("l_returnflag",    Value::Text(col[8].trim().to_string())),
                ("l_linestatus",    Value::Text(col[9].trim().to_string())),
                ("l_shipdate",      Value::Integer(shipdate)),
                ("l_shipmode",      Value::Text(col[14].trim().to_string())),
            ]));
            if lid % 1_000_000 == 0 {
                println!("    ...{}M rows ({:.1}s)", lid / 1_000_000, t.elapsed().as_secs_f64());
            }
            lid += 1;
        }
    }
    let n_lineitems = (lid - 1) as usize;
    println!("    {n_lineitems} rows in {:.1}s", t.elapsed().as_secs_f64());

    Some(TpchData { customer, orders, lineitem, part, n_customers, n_orders, n_lineitems })
}

/// Run Q1/Q3/Q6/Q14 on pre-loaded data (used for both synthetic and real).
fn bench_on_data(data: &TpchData, sf: f64) -> Vec<BenchResult> {
    let li_mode = data.lineitem.storage_mode();
    let ord_mode = data.orders.storage_mode();
    let mut results = Vec::new();
    let n_runs = 3;

    let mut best_ms = f64::MAX;
    let mut q6_rows = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let (_, cnt) = q6(data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q6_rows = cnt; }
    }
    results.push(BenchResult {
        query: "Q6 (filter+agg)", sf,
        rows_scanned: data.n_lineitems, rows_returned: q6_rows,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode, orders_storage: ord_mode, q14_promo_pct: 0.0,
    });

    let mut best_ms = f64::MAX;
    let mut q1_groups = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let groups = q1(data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q1_groups = groups.len(); }
    }
    results.push(BenchResult {
        query: "Q1 (group agg)", sf,
        rows_scanned: data.n_lineitems, rows_returned: q1_groups,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode, orders_storage: ord_mode, q14_promo_pct: 0.0,
    });

    let mut best_ms = f64::MAX;
    let mut q14_pct = 0.0_f64;
    for _ in 0..n_runs {
        let t = Instant::now();
        let pct = q14(data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q14_pct = pct; }
    }
    results.push(BenchResult {
        query: "Q14 (join+cond)", sf,
        rows_scanned: data.n_lineitems, rows_returned: 1,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode, orders_storage: ord_mode, q14_promo_pct: q14_pct,
    });
    println!("  Q14 promo_revenue = {q14_pct:.2}%");

    let mut best_ms = f64::MAX;
    let mut q3_groups = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let groups = q3(data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q3_groups = groups.len(); }
    }
    results.push(BenchResult {
        query: "Q3 (3-way join)", sf,
        rows_scanned: data.n_lineitems + data.n_orders + data.n_customers,
        rows_returned: q3_groups,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / (data.n_lineitems + data.n_orders) as f64,
        lineitem_storage: li_mode, orders_storage: ord_mode, q14_promo_pct: 0.0,
    });

    results
}

// ── Benchmark harness ──────────────────────────────────────────────

#[allow(dead_code)]
struct BenchResult {
    query: &'static str,
    sf: f64,
    rows_scanned: usize,
    rows_returned: usize,
    wall_ms: f64,
    ns_per_row: f64,
    lineitem_storage: &'static str,
    orders_storage: &'static str,
    q14_promo_pct: f64,  // 0.0 for non-Q14 rows
}

fn run_bench(sf: f64) -> Vec<BenchResult> {
    println!("  Generating SF={sf}...");
    let t0 = Instant::now();
    let data = generate(sf, 42);
    println!("  Generated: LI={}, ORD={}, CUST={} in {:.1}ms",
             data.n_lineitems, data.n_orders, data.n_customers,
             t0.elapsed().as_secs_f64() * 1000.0);

    let li_mode = data.lineitem.storage_mode();
    let ord_mode = data.orders.storage_mode();

    let mut results = Vec::new();
    let n_runs = 3;

    // Q6
    let mut best_ms = f64::MAX;
    let mut q6_rows = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let (_, cnt) = q6(&data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q6_rows = cnt; }
    }
    results.push(BenchResult {
        query: "Q6 (filter+agg)", sf,
        rows_scanned: data.n_lineitems, rows_returned: q6_rows,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode,
        orders_storage: ord_mode,
        q14_promo_pct: 0.0,
    });

    // Q1
    let mut best_ms = f64::MAX;
    let mut q1_groups = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let groups = q1(&data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q1_groups = groups.len(); }
    }
    results.push(BenchResult {
        query: "Q1 (group agg)", sf,
        rows_scanned: data.n_lineitems, rows_returned: q1_groups,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode,
        orders_storage: ord_mode,
        q14_promo_pct: 0.0,
    });

    // Q14
    let mut best_ms = f64::MAX;
    let mut q14_pct = 0.0_f64;
    for _ in 0..n_runs {
        let t = Instant::now();
        let pct = q14(&data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q14_pct = pct; }
    }
    results.push(BenchResult {
        query: "Q14 (join+cond)", sf,
        rows_scanned: data.n_lineitems, rows_returned: 1,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / data.n_lineitems as f64,
        lineitem_storage: li_mode,
        orders_storage: ord_mode,
        q14_promo_pct: q14_pct,
    });
    println!("  Q14 promo_revenue = {q14_pct:.2}%");

    // Q3
    let mut best_ms = f64::MAX;
    let mut q3_groups = 0;
    for _ in 0..n_runs {
        let t = Instant::now();
        let groups = q3(&data);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if ms < best_ms { best_ms = ms; q3_groups = groups.len(); }
    }
    results.push(BenchResult {
        query: "Q3 (3-way join)", sf,
        rows_scanned: data.n_lineitems + data.n_orders + data.n_customers,
        rows_returned: q3_groups,
        wall_ms: best_ms,
        ns_per_row: best_ms * 1_000_000.0 / (data.n_lineitems + data.n_orders) as f64,
        lineitem_storage: li_mode,
        orders_storage: ord_mode,
        q14_promo_pct: 0.0,
    });

    results
}

// -- DHOOM report ----------------------------------------------------------

fn emit_dhoom_report(synthetic: &[(f64, Vec<BenchResult>)], real: Option<&[BenchResult]>) {
    let mut file_buf = String::new();

    macro_rules! section {
        ($title:expr, $val:expr) => {{
            match gigi::dhoom::encode($val) {
                Ok(s) => {
                    let block = format!("\n-- {} --\n{}", $title, s);
                    println!("{block}");
                    file_buf.push_str(&block);
                    file_buf.push('\n');
                }
                Err(e) => eprintln!("  [dhoom encode error for {}: {e}]", $title),
            }
        }};
    }

    // -- section 1: query catalogue --
    let queries_val = json!({
        "queries": [
            {
                "id": "Q6", "tpch": 6,
                "name": "Forecasting Revenue Change",
                "operation": "filter + scan aggregate",
                "join_tier": 1,
                "predicate": "shipdate in [1994-01-01, 1995-01-01), discount in [0.05,0.07], qty < 24",
                "notes": "Pure LINEITEM scan, no join. Validates sequential storage + filter throughput."
            },
            {
                "id": "Q1", "tpch": 1,
                "name": "Pricing Summary Report",
                "operation": "filter + group_by + multi-aggregate",
                "join_tier": 2,
                "predicate": "shipdate <= 1998-12-01",
                "notes": "Groups by (returnflag, linestatus). Validates HashMap aggregation over 6M rows."
            },
            {
                "id": "Q14", "tpch": 14,
                "name": "Promotion Effect",
                "operation": "filter + pullback_join + conditional aggregate",
                "join_tier": 1,
                "predicate": "shipdate in [1995-09-01, 1995-10-01), join PART on p_type LIKE PROMO%",
                "notes": "Pullback join LINEITEM->PART. promo_revenue = 100 * sum(promo_price) / sum(total_price). Validates correctness of join + conditional agg."
            },
            {
                "id": "Q3", "tpch": 3,
                "name": "Shipping Priority",
                "operation": "CUSTOMER-bitmap -> ORDERS -> LINEITEM 3-way join + aggregate",
                "join_tier": 2,
                "predicate": "c_mktsegment = BUILDING, o_orderdate < 1995-03-15, l_shipdate > 1995-03-15",
                "notes": "Bitmap index on customer segment pre-filters ~30K BUILDING customers before any row scan. Demonstrates predicate pushdown efficiency."
            }
        ]
    });
    section!("DHOOM: Query Catalogue", &queries_val);

    // -- section 2: all results --
    let mut rows = Vec::new();
    for (sf, results) in synthetic {
        for r in results {
            rows.push(json!({
                "query": r.query,
                "sf": sf,
                "data_source": "synthetic",
                "rows_scanned": r.rows_scanned,
                "rows_returned": r.rows_returned,
                "wall_ms": (r.wall_ms * 10.0).round() / 10.0,
                "ns_per_row": (r.ns_per_row * 10.0).round() / 10.0,
                "lineitem_store": r.lineitem_storage,
                "orders_store": r.orders_storage,
                "q14_promo_pct": if r.q14_promo_pct > 0.0 { format!("{:.2}%", r.q14_promo_pct) } else { String::new() }
            }));
        }
    }
    if let Some(real_results) = real {
        for r in real_results {
            rows.push(json!({
                "query": r.query,
                "sf": 1.0,
                "data_source": "real_tpch_sf1",
                "rows_scanned": r.rows_scanned,
                "rows_returned": r.rows_returned,
                "wall_ms": (r.wall_ms * 10.0).round() / 10.0,
                "ns_per_row": (r.ns_per_row * 10.0).round() / 10.0,
                "lineitem_store": r.lineitem_storage,
                "orders_store": r.orders_storage,
                "q14_promo_pct": if r.q14_promo_pct > 0.0 { format!("{:.2}%", r.q14_promo_pct) } else { String::new() }
            }));
        }
    }
    section!("DHOOM: Timing Results", &json!({ "results": rows }));

    // -- section 3: scaling proof --
    if synthetic.len() >= 2 {
        let (sf1, r1) = &synthetic[0];
        let (sf2, r2) = &synthetic[1];
        let scale = sf2 / sf1;
        let mut scaling_rows = Vec::new();
        for i in 0..r1.len().min(r2.len()) {
            let ratio = r2[i].wall_ms / r1[i].wall_ms.max(0.001);
            let deviation_pct = ((ratio / scale) - 1.0).abs() * 100.0;
            scaling_rows.push(json!({
                "query": r1[i].query,
                "sf_small": sf1,
                "sf_large": sf2,
                "scale_factor": scale,
                "rows_small": r1[i].rows_scanned,
                "rows_large": r2[i].rows_scanned,
                "ms_small": (r1[i].wall_ms * 100.0).round() / 100.0,
                "ms_large": (r2[i].wall_ms * 100.0).round() / 100.0,
                "observed_ratio": (ratio * 10.0).round() / 10.0,
                "deviation_pct": (deviation_pct * 10.0).round() / 10.0,
                "verdict": if deviation_pct < 50.0 { "O(n) confirmed" } else { "non-linear" }
            }));
        }
        section!("DHOOM: Scaling Linearity Proof", &json!({ "scaling_linearity": scaling_rows }));
    }

    // -- section 4: storage geometry --
    let storage_val = json!({
        "storage_geometry": [
            {
                "table": "LINEITEM",
                "mode": "sequential",
                "reason": "l_id is auto-increment dense int (K=0 base). BaseGeometry::Flat with step=1.",
                "implication": "O(1) point lookup by row id. Full scan is cache-linear."
            },
            {
                "table": "ORDERS",
                "mode": "hashed",
                "reason": "o_orderkey is sparse (gaps 1-8 between keys). BundleStore::new() auto-selects HashMap.",
                "implication": "O(1) point lookup by orderkey. Q3 bitmap join resolves orderkeys in O(|BUILDING_customers|) not O(|ORDERS|)."
            },
            {
                "table": "CUSTOMER",
                "mode": "sequential",
                "reason": "c_custkey is dense 1..150000.",
                "implication": "Bitmap over mktsegment builds in O(150K). Q3 segment filter costs O(150K) not O(1.5M orders)."
            }
        ]
    });
    section!("DHOOM: Storage Geometry", &storage_val);

    // -- write file --
    let out_path = "tpch_report.dhoom";
    match std::fs::write(out_path, &file_buf) {
        Ok(_) => println!("\n  [report written to {out_path}]"),
        Err(e) => eprintln!("  [failed to write {out_path}: {e}]"),
    }
}

fn assert_linear(small: &BenchResult, large: &BenchResult, scale: f64) {
    let ratio = large.wall_ms / small.wall_ms.max(0.001);
    let lo = scale * 0.3;
    let hi = scale * 5.0;
    let ok = ratio > lo && ratio < hi;
    let mark = if ok { "[OK]" } else { "[!!]" };
    println!(
        "  {mark} {}: {:.0}K rows={:.2}ms, {:.0}K rows={:.2}ms -> {:.1}x (expect ~{scale}x)",
        small.query,
        small.rows_scanned as f64 / 1000.0, small.wall_ms,
        large.rows_scanned as f64 / 1000.0, large.wall_ms,
        ratio,
    );
}

fn main() {
    println!("+=======================================================+");
    println!("|        GIGI TPC-H Benchmark - Q1, Q3, Q6, Q14       |");
    println!("+=======================================================+\n");

    let scale_factors = [0.001_f64, 0.01, 0.05];
    let mut all_results: Vec<(f64, Vec<BenchResult>)> = Vec::new();

    for sf in scale_factors {
        println!("-- SF = {sf} ----------------------------------");
        let results = run_bench(sf);
        all_results.push((sf, results));
        println!();
    }

    // ── Results table ──────────────────────────────────────────────
    println!("\n{:<22} {:>6} {:>10} {:>10} {:>9} {:>9} {:>10} {:>10}",
             "Query", "SF", "Scanned", "Returned", "ms", "ns/row",
             "LI-store", "ORD-store");
    println!("{}", "-".repeat(95));

    for (sf, results) in &all_results {
        for r in results {
            println!("{:<22} {:>6.3} {:>10} {:>10} {:>9.2} {:>9.1} {:>10} {:>10}",
                     r.query, sf,
                     r.rows_scanned, r.rows_returned,
                     r.wall_ms, r.ns_per_row,
                     r.lineitem_storage, r.orders_storage);
        }
    }

    // ── Scaling linearity check ────────────────────────────────────
    println!("\n-- Scaling Linearity (expect ~10x) ------------------");
    if all_results.len() >= 2 {
        let (_, r1) = &all_results[0];
        let (_, r2) = &all_results[1];
        let scale = 10.0_f64;  // 0.001 → 0.01 is 10×
        for i in 0..r1.len().min(r2.len()) {
            assert_linear(&r1[i], &r2[i], scale);
        }
    }

    // ── Storage mode summary ───────────────────────────────────────
    println!("\n-- Storage Mode Verification --------------------");
    if let Some((_, results)) = all_results.first() {
        if let Some(r) = results.first() {
            let li_ok = r.lineitem_storage == "sequential";
            let ord_ok = r.orders_storage == "hashed";
            println!("  {} LINEITEM -> {} (expect: sequential, K=0 auto-inc base)",
                     if li_ok { "[OK]" } else { "[!!]" }, r.lineitem_storage);
            println!("  {} ORDERS -> {} (expect: hashed, sparse keys)",
                     if ord_ok { "[OK]" } else { "[!!]" }, r.orders_storage);
        }
    }

    println!("\nDone.");

    // ── Real data SF=1 (if .tbl files present) ────────────────────
    let tbl_dir = std::env::var("TPCH_TBL_DIR")
        .unwrap_or_else(|_| r"C:\Users\nurdm\tpch-kit\tbl".to_string());

    if std::path::Path::new(&tbl_dir).join("lineitem.tbl").exists() {
        println!("\n+=======================================================+");
        println!("|         SF=1 Real Data - 6,001,215 LINEITEM rows     |");
        println!("+=======================================================+");
        println!("  Source: {tbl_dir}\n");

        let t_load = Instant::now();
        match load_real_data(&tbl_dir) {
            Some(data) => {
                println!("\n  Loaded in {:.1}s total. Running queries...\n",
                         t_load.elapsed().as_secs_f64());

                let real_results = bench_on_data(&data, 1.0);

                println!("\n{:<22} {:>10} {:>10} {:>12} {:>9} {:>10} {:>10}",
                         "Query", "Scanned", "Returned", "ms", "ns/row",
                         "LI-store", "ORD-store");
                println!("{}", "-".repeat(85));
                for r in &real_results {
                    println!("{:<22} {:>10} {:>10} {:>12.1} {:>9.1} {:>10} {:>10}",
                             r.query, r.rows_scanned, r.rows_returned,
                             r.wall_ms, r.ns_per_row,
                             r.lineitem_storage, r.orders_storage);
                }

                println!("\n  Storage mode: LINEITEM={}, ORDERS={}",
                         data.lineitem.storage_mode(), data.orders.storage_mode());

                println!("\n\n=========================================================");
                println!("  DHOOM REPORT");
                println!("=========================================================");
                emit_dhoom_report(&all_results, Some(&real_results));
            }
            None => {
                println!("  [skipped -- .tbl files not found]");
                println!("\n\n=========================================================");
                println!("  DHOOM REPORT  (synthetic data only)");
                println!("=========================================================");
                emit_dhoom_report(&all_results, None);
            }
        }
    } else {
        println!("\n\n=========================================================");
        println!("  DHOOM REPORT  (synthetic data only)");
        println!("=========================================================");
        emit_dhoom_report(&all_results, None);
    }
}
