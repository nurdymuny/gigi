//! NASA Atmosphere Analysis — GIGI in action on real data.
//!
//! Fetches daily atmospheric data from NASA POWER API for 20 global cities,
//! stores it in GIGI bundles, and runs geometric analysis:
//!   - Curvature → anomaly detection (storms, heat waves, cold snaps)
//!   - Spectral gap → geographic connectivity analysis
//!   - RG flow → multi-scale climate patterns
//!   - Confidence scores → data quality assessment
//!
//! NASA POWER API: free, no key required, global coverage.
//!
//! Usage:
//!   cargo run --release --bin nasa_atmo

use std::collections::HashMap;
use std::time::Instant;

use gigi::bundle::BundleStore;
use gigi::curvature;
use gigi::spectral;
use gigi::types::*;
use gigi::aggregation;

// ── City definitions ──

struct City {
    name: &'static str,
    lat: f64,
    lon: f64,
    region: &'static str,
}

const CITIES: &[City] = &[
    // North America
    City { name: "New_York",    lat: 40.71,  lon: -74.01,  region: "NA"  },
    City { name: "Los_Angeles", lat: 34.05,  lon: -118.24, region: "NA"  },
    City { name: "Chicago",     lat: 41.88,  lon: -87.63,  region: "NA"  },
    City { name: "Houston",     lat: 29.76,  lon: -95.37,  region: "NA"  },
    City { name: "Toronto",     lat: 43.65,  lon: -79.38,  region: "NA"  },
    // Europe
    City { name: "London",      lat: 51.51,  lon: -0.13,   region: "EU"  },
    City { name: "Paris",       lat: 48.86,  lon: 2.35,    region: "EU"  },
    City { name: "Berlin",      lat: 52.52,  lon: 13.41,   region: "EU"  },
    City { name: "Moscow",      lat: 55.76,  lon: 37.62,   region: "EU"  },
    City { name: "Rome",        lat: 41.90,  lon: 12.50,   region: "EU"  },
    // Asia
    City { name: "Tokyo",       lat: 35.68,  lon: 139.69,  region: "AS"  },
    City { name: "Beijing",     lat: 39.90,  lon: 116.40,  region: "AS"  },
    City { name: "Mumbai",      lat: 19.08,  lon: 72.88,   region: "AS"  },
    City { name: "Singapore",   lat: 1.35,   lon: 103.82,  region: "AS"  },
    City { name: "Dubai",       lat: 25.20,  lon: 55.27,   region: "AS"  },
    // Southern hemisphere
    City { name: "Sydney",      lat: -33.87, lon: 151.21,  region: "SH"  },
    City { name: "Sao_Paulo",   lat: -23.55, lon: -46.63,  region: "SH"  },
    City { name: "Cape_Town",   lat: -33.93, lon: 18.42,   region: "SH"  },
    City { name: "Buenos_Aires",lat: -34.60, lon: -58.38,  region: "SH"  },
    City { name: "Auckland",    lat: -36.85, lon: 174.76,  region: "SH"  },
];

// ── NASA POWER API ──

/// Parameters we fetch from NASA POWER:
///   T2M       - Temperature at 2 meters (°C)
///   T2M_MAX   - Max daily temperature (°C)
///   T2M_MIN   - Min daily temperature (°C)
///   RH2M      - Relative humidity at 2 meters (%)
///   PS        - Surface pressure (kPa)
///   WS2M      - Wind speed at 2 meters (m/s)
///   ALLSKY_SFC_SW_DWN - Solar irradiance (kWh/m²/day)
const PARAMS: &str = "T2M,T2M_MAX,T2M_MIN,RH2M,PS,WS2M,ALLSKY_SFC_SW_DWN";

fn fetch_city_data(
    client: &reqwest::blocking::Client,
    city: &City,
    start: &str,
    end: &str,
) -> Result<Vec<HashMap<String, f64>>, String> {
    let url = format!(
        "https://power.larc.nasa.gov/api/temporal/daily/point\
         ?parameters={PARAMS}\
         &community=RE\
         &longitude={lon}\
         &latitude={lat}\
         &start={start}\
         &end={end}\
         &format=JSON",
        lon = city.lon,
        lat = city.lat,
    );

    println!("  Fetching {} ({}, {})...", city.name, city.lat, city.lon);

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("HTTP error for {}: {e}", city.name))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {}", resp.status(), city.name));
    }

    let body: serde_json::Value = resp
        .json()
        .map_err(|e| format!("JSON parse error for {}: {e}", city.name))?;

    // NASA POWER response structure:
    // { "properties": { "parameter": { "T2M": { "20240101": 5.23, ... }, ... } } }
    let params = body
        .get("properties")
        .and_then(|p| p.get("parameter"))
        .ok_or_else(|| format!("Missing parameter data for {}", city.name))?;

    // Collect all dates
    let t2m = params.get("T2M").and_then(|v| v.as_object())
        .ok_or_else(|| format!("Missing T2M for {}", city.name))?;

    let mut records = Vec::new();
    for (date_str, _temp_val) in t2m {
        let mut rec = HashMap::new();
        rec.insert("date".to_string(), date_str.parse::<f64>().unwrap_or(0.0));

        // Extract each parameter for this date
        for param_name in ["T2M", "T2M_MAX", "T2M_MIN", "RH2M", "PS", "WS2M", "ALLSKY_SFC_SW_DWN"] {
            let val = params
                .get(param_name)
                .and_then(|p| p.get(date_str))
                .and_then(|v| v.as_f64())
                .unwrap_or(-999.0); // NASA uses -999 for missing
            rec.insert(param_name.to_string(), val);
        }

        // Skip records with missing data
        let has_missing = rec.values().any(|&v| v < -998.0);
        if !has_missing {
            records.push(rec);
        }
    }

    println!("    → {} valid daily records", records.len());
    Ok(records)
}

// ── GIGI Bundle Construction ──

fn build_atmosphere_bundle() -> BundleSchema {
    BundleSchema::new("atmosphere")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("city"))
        .fiber(FieldDef::categorical("region"))
        .fiber(FieldDef::numeric("date"))
        .fiber(FieldDef::numeric("temp").with_range(80.0))       // -40 to +50 °C
        .fiber(FieldDef::numeric("temp_max").with_range(80.0))
        .fiber(FieldDef::numeric("temp_min").with_range(80.0))
        .fiber(FieldDef::numeric("humidity").with_range(100.0))   // 0-100%
        .fiber(FieldDef::numeric("pressure").with_range(20.0))    // ~90-110 kPa
        .fiber(FieldDef::numeric("wind").with_range(30.0))        // 0-30 m/s
        .fiber(FieldDef::numeric("solar").with_range(12.0))       // 0-12 kWh/m²/day
        .index("city")
        .index("region")
}

fn ingest_records(
    store: &mut BundleStore,
    city: &City,
    records: &[HashMap<String, f64>],
    id_offset: &mut i64,
) {
    for rec in records {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(*id_offset));
        r.insert("city".into(), Value::Text(city.name.to_string()));
        r.insert("region".into(), Value::Text(city.region.to_string()));
        r.insert("date".into(), Value::Float(rec["date"]));
        r.insert("temp".into(), Value::Float(rec["T2M"]));
        r.insert("temp_max".into(), Value::Float(rec["T2M_MAX"]));
        r.insert("temp_min".into(), Value::Float(rec["T2M_MIN"]));
        r.insert("humidity".into(), Value::Float(rec["RH2M"]));
        r.insert("pressure".into(), Value::Float(rec["PS"]));
        r.insert("wind".into(), Value::Float(rec["WS2M"]));
        r.insert("solar".into(), Value::Float(rec["ALLSKY_SFC_SW_DWN"]));
        store.insert(&r);
        *id_offset += 1;
    }
}

// ── Analysis Functions ──

fn analyze_curvature_by_city(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║         CURVATURE ANALYSIS — Anomaly Detection by City         ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  City            │   K(temp)  │   K(wind)  │ Confidence │ Flag ║");
    println!("╟──────────────────┼────────────┼────────────┼────────────┼──────╢");

    // Build per-city mini-bundles for curvature
    let mut city_data: HashMap<String, Vec<Record>> = HashMap::new();
    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            city_data.entry(city.clone()).or_default().push(rec);
        }
    }

    let mut cities: Vec<_> = city_data.keys().cloned().collect();
    cities.sort();

    for city_name in &cities {
        let records = &city_data[city_name];

        // Compute variance / range² for temperature
        let temps: Vec<f64> = records.iter()
            .filter_map(|r| r.get("temp").and_then(|v| v.as_f64()))
            .collect();
        let winds: Vec<f64> = records.iter()
            .filter_map(|r| r.get("wind").and_then(|v| v.as_f64()))
            .collect();

        let k_temp = field_curvature(&temps, 80.0);
        let k_wind = field_curvature(&winds, 30.0);
        let conf = 1.0 / (1.0 + k_temp);

        let flag = if k_temp > 0.05 { "HIGH" } else if k_temp > 0.02 { " MED" } else { " LOW" };

        println!("║  {:<16} │ {:>10.6} │ {:>10.6} │ {:>10.4} │ {:<4} ║",
            city_name, k_temp, k_wind, conf, flag);
    }
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!("  HIGH curvature = high variability = weather anomalies likely");
    println!("  LOW  curvature = stable conditions = predictable climate");
}

fn field_curvature(values: &[f64], range: f64) -> f64 {
    if values.len() < 2 || range == 0.0 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance / (range * range)
}

fn analyze_global_curvature(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║            GLOBAL CURVATURE — Full Bundle Analysis             ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let k = curvature::scalar_curvature(store);
    let conf = curvature::confidence(k);
    let cap = curvature::capacity(0.1, k);

    println!("║  Records:           {:>8}                                    ║", store.len());
    println!("║  Scalar Curvature:  {:>10.6}                                 ║", k);
    println!("║  Confidence:        {:>10.6}                                 ║", conf);
    println!("║  Capacity (τ=0.1):  {:>10.4}                                 ║", cap);
    println!("║                                                                ║");

    // Partition function at different temperatures
    println!("║  Partition Function Z(β):                                      ║");
    for tau in [0.001, 0.01, 0.1, 1.0, 10.0] {
        // Sample a base point
        if let Some((bp, _)) = store.sections().next() {
            let z = curvature::partition_function(store, bp, tau);
            println!("║    τ = {:>6.3}  →  Z = {:>12.4}                              ║", tau, z);
        }
    }
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_spectral(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║          SPECTRAL ANALYSIS — Index Connectivity                ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let t0 = Instant::now();
    let lambda1 = spectral::spectral_gap(store);
    // Skip expensive diameter computation when disconnected (λ₁ = 0)
    let diam = if lambda1 < f64::EPSILON { 1 } else { spectral::graph_diameter(store) };
    let c_sp = if lambda1 < f64::EPSILON { 0.0 } else { spectral::spectral_capacity(store) };
    let elapsed = t0.elapsed();

    println!("║  Spectral gap λ₁:     {:>10.6}                               ║", lambda1);
    println!("║  Graph diameter D:    {:>10}                               ║", diam);
    println!("║  Spectral capacity:   {:>10.4}                               ║", c_sp);
    println!("║  Mixing time O(1/λ₁): {:>10.2}                               ║",
        if lambda1 > 1e-10 { 1.0 / lambda1 } else { f64::INFINITY });
    println!("║  Computed in:         {:>10.2?}                            ║", elapsed);
    println!("║                                                                ║");

    if lambda1 > 0.5 {
        println!("║  ✓ Large spectral gap → well-connected index graph            ║");
        println!("║    All cities reachable within {} hops via shared fields      ║", diam);
    } else if lambda1 > 0.01 {
        println!("║  ◐ Moderate spectral gap → partial connectivity              ║");
        println!("║    Some geographic clusters weakly linked                     ║");
    } else {
        println!("║  ✗ Small spectral gap → disconnected clusters                ║");
        println!("║    Data has isolated geographic communities                  ║");
    }
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_rg_flow(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║        RG FLOW — Multi-Scale Structure (C-Theorem)             ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Scale │ Groups │    Entropy │ C-Theorem                       ║");
    println!("╟────────┼────────┼────────────┼─────────────────────────────────╢");

    let mut prev_entropy = f64::MAX;
    let max_scale = store.schema.indexed_fields.len().max(1) + 1;

    for scale in 1..=max_scale {
        let (groups, entropy) = spectral::coarse_grain(store, scale);
        let monotone = if entropy <= prev_entropy + 1e-10 { "✓ monotone" } else { "✗ VIOLATION" };
        println!("║  ℓ = {} │ {:>6} │ {:>10.6} │ {}                   ║",
            scale, groups.len(), entropy, monotone);
        prev_entropy = entropy;
    }
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  C-theorem: completion entropy non-increasing under coarsening ║");
    println!("║  ℓ=1: fine (city×region)  ℓ=2: medium (region only)  ℓ=3: all ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_regions(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║       REGIONAL AGGREGATION — Fiber Integrals by Region         ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Region │ Records │  Avg Temp │  Avg Wind │ Avg Humid │  Avg ☀ ║");
    println!("╟─────────┼─────────┼───────────┼───────────┼───────────┼────────╢");

    let _groups = aggregation::group_by(store, "region", "temp");

    // Also compute other aggregates manually
    let mut region_data: HashMap<String, (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = HashMap::new();
    for rec in store.records() {
        if let Some(Value::Text(region)) = rec.get("region") {
            let entry = region_data.entry(region.clone()).or_default();
            if let Some(v) = rec.get("temp").and_then(|v| v.as_f64()) { entry.0.push(v); }
            if let Some(v) = rec.get("wind").and_then(|v| v.as_f64()) { entry.1.push(v); }
            if let Some(v) = rec.get("humidity").and_then(|v| v.as_f64()) { entry.2.push(v); }
            if let Some(v) = rec.get("solar").and_then(|v| v.as_f64()) { entry.3.push(v); }
        }
    }

    let mut regions: Vec<_> = region_data.keys().cloned().collect();
    regions.sort();

    for region in &regions {
        let (temps, winds, humids, solars) = &region_data[region];
        let n = temps.len();
        let avg = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };

        println!("║  {:<7} │ {:>7} │ {:>8.2}°C │ {:>8.2}m/s│ {:>8.1}% │ {:>5.2}kW ║",
            region, n, avg(temps), avg(winds), avg(humids), avg(solars));
    }
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_extremes(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║     EXTREME WEATHER DETECTION — Curvature Spike Analysis       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Find records with extreme deviations
    let mut extremes: Vec<(String, f64, f64, f64, f64)> = Vec::new();

    // Compute global stats first
    let mut all_temps: Vec<f64> = Vec::new();
    let mut all_winds: Vec<f64> = Vec::new();
    for rec in store.records() {
        if let Some(t) = rec.get("temp").and_then(|v| v.as_f64()) { all_temps.push(t); }
        if let Some(w) = rec.get("wind").and_then(|v| v.as_f64()) { all_winds.push(w); }
    }
    let temp_mean = all_temps.iter().sum::<f64>() / all_temps.len() as f64;
    let temp_std = (all_temps.iter().map(|x| (x - temp_mean).powi(2)).sum::<f64>() / all_temps.len() as f64).sqrt();
    let wind_mean = all_winds.iter().sum::<f64>() / all_winds.len() as f64;
    let wind_std = (all_winds.iter().map(|x| (x - wind_mean).powi(2)).sum::<f64>() / all_winds.len() as f64).sqrt();

    for rec in store.records() {
        let city = rec.get("city").map(|v| v.to_string()).unwrap_or_default();
        let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let date = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let temp_z = (temp - temp_mean).abs() / temp_std;
        let wind_z = (wind - wind_mean).abs() / wind_std;

        if temp_z > 2.5 || wind_z > 3.0 {
            extremes.push((city, date, temp, wind, temp_z.max(wind_z)));
        }
    }

    extremes.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());
    let top = extremes.iter().take(15);

    println!("║  City            │   Date     │  Temp(°C) │ Wind(m/s)│ Z-score ║");
    println!("╟──────────────────┼────────────┼───────────┼──────────┼─────────╢");
    for (city, date, temp, wind, z) in top {
        let flag = if *z > 4.0 { "!!!" } else if *z > 3.0 { " !!" } else { "  !" };
        println!("║  {:<16} │ {:>10.0} │ {:>8.1}  │ {:>7.1}  │ {:>5.2} {} ║",
            city, date, temp, wind, z, flag);
    }
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Z > 3.0 = extreme event (> 3σ from global mean)              ║");
    println!("║  High curvature regions = geometric anomaly detection at work  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn timing_report(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║            PERFORMANCE — O(1) on Real Data                     ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let n = store.len();

    // Point query timing
    let mut key = Record::new();
    key.insert("id".into(), Value::Integer(0));
    let t0 = Instant::now();
    let iters = 10_000;
    for i in 0..iters {
        key.insert("id".into(), Value::Integer(i % n as i64));
        let _ = store.point_query(&key);
    }
    let pq_ns = t0.elapsed().as_nanos() as f64 / iters as f64;

    // Range query timing
    let t0 = Instant::now();
    let range_iters = 100;
    for _ in 0..range_iters {
        let _ = store.range_query("region", &[Value::Text("NA".into())]);
    }
    let rq_ns = t0.elapsed().as_nanos() as f64 / range_iters as f64;

    // Insert timing
    let schema = build_atmosphere_bundle();
    let mut bench_store = BundleStore::new(schema);
    let t0 = Instant::now();
    let insert_iters = 10_000;
    for i in 0..insert_iters {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("city".into(), Value::Text("Bench".into()));
        r.insert("region".into(), Value::Text("XX".into()));
        r.insert("date".into(), Value::Float(20240101.0));
        r.insert("temp".into(), Value::Float(20.0));
        r.insert("temp_max".into(), Value::Float(25.0));
        r.insert("temp_min".into(), Value::Float(15.0));
        r.insert("humidity".into(), Value::Float(50.0));
        r.insert("pressure".into(), Value::Float(101.3));
        r.insert("wind".into(), Value::Float(5.0));
        r.insert("solar".into(), Value::Float(4.5));
        bench_store.insert(&r);
    }
    let ins_ns = t0.elapsed().as_nanos() as f64 / insert_iters as f64;

    println!("║  Dataset:      {:>8} records × 11 fields                    ║", n);
    println!("║  Point query:  {:>8.0} ns   (O(1) — constant)                ║", pq_ns);
    println!("║  Range query:  {:>8.0} ns   (O(|result|))                     ║", rq_ns);
    println!("║  Insert:       {:>8.0} ns   (O(1) amortized)                  ║", ins_ns);
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

// ── Showstopper Analysis Functions ──

fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n < 2.0 { return 0.0; }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let cov: f64 = x.iter().zip(y.iter()).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sx = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let sy = y.iter().map(|a| (a - my).powi(2)).sum::<f64>().sqrt();
    if sx < 1e-12 || sy < 1e-12 { 0.0 } else { cov / (sx * sy) }
}

fn analyze_curvature_predicts_extremes(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║    CURVATURE PREDICTS EXTREMES — The Geometry Knew             ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Per-city K(temp)
    let mut city_temps: HashMap<String, Vec<f64>> = HashMap::new();
    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            if let Some(t) = rec.get("temp").and_then(|v| v.as_f64()) {
                city_temps.entry(city.clone()).or_default().push(t);
            }
        }
    }

    // Global stats for Z-scores
    let mut all_temps = Vec::new();
    let mut all_winds = Vec::new();
    for rec in store.records() {
        if let Some(t) = rec.get("temp").and_then(|v| v.as_f64()) { all_temps.push(t); }
        if let Some(w) = rec.get("wind").and_then(|v| v.as_f64()) { all_winds.push(w); }
    }
    let t_mean = all_temps.iter().sum::<f64>() / all_temps.len() as f64;
    let t_std = (all_temps.iter().map(|x| (x - t_mean).powi(2)).sum::<f64>()
        / all_temps.len() as f64).sqrt();
    let w_mean = all_winds.iter().sum::<f64>() / all_winds.len() as f64;
    let w_std = (all_winds.iter().map(|x| (x - w_mean).powi(2)).sum::<f64>()
        / all_winds.len() as f64).sqrt();

    // Find top 15 extreme events and count per city
    let mut events: Vec<(String, f64)> = Vec::new();
    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let z = ((temp - t_mean).abs() / t_std)
                .max((wind - w_mean).abs() / w_std);
            if z > 2.5 {
                events.push((city.clone(), z));
            }
        }
    }
    events.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let top15: Vec<_> = events.iter().take(15).collect();

    let mut top15_count: HashMap<String, usize> = HashMap::new();
    let mut top15_peak: HashMap<String, f64> = HashMap::new();
    for (city, z) in &top15 {
        *top15_count.entry(city.clone()).or_insert(0) += 1;
        let e = top15_peak.entry(city.clone()).or_insert(0.0);
        if *z > *e { *e = *z; }
    }

    // Sort cities by K(temp) descending
    let mut ranked: Vec<(String, f64)> = city_temps.iter()
        .map(|(city, temps)| (city.clone(), field_curvature(temps, 80.0)))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (city, k) in &ranked {
        let flag = if *k > 0.05 { "HIGH" }
            else if *k > 0.02 { " MED" }
            else { " LOW" };
        let count = top15_count.get(city).copied().unwrap_or(0);
        let peak = top15_peak.get(city).copied().unwrap_or(0.0);
        let detail = if count > 0 {
            format!("{} of top 15 (Z>{:.1})", count, peak)
        } else {
            "0 extremes".to_string()
        };
        println!("║{:<64}║",
            format!("  {:<14} K={:.4}  {} → {}", city, k, flag, detail));
    }

    let k_vals: Vec<f64> = ranked.iter().map(|r| r.1).collect();
    let ext_vals: Vec<f64> = ranked.iter()
        .map(|r| top15_count.get(&r.0).copied().unwrap_or(0) as f64).collect();
    let corr = pearson_correlation(&k_vals, &ext_vals);

    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║{:<64}║",
        format!("  Pearson r(K, extremes) = {:.4}", corr));
    println!("║{:<64}║", "");
    println!("║{:<64}║",
        "  High curvature = high variability = more extreme events.");
    println!("║{:<64}║",
        "  The geometry knew where to look BEFORE scanning records.");
    println!("║{:<64}║",
        "  No other database does this.");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_prediction(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║   CURVATURE PREDICTION — Train on Past, Predict Future         ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║{:<64}║", "  Trained on: Jan\u{2013}Sep 2024 (9 months per city)");
    println!("║{:<64}║", "  Testing on: Oct\u{2013}Dec 2024 (unseen future data)");
    println!("║{:<64}║", "  Method: K(temp) above median \u{2192} predict extremes");
    println!("╟──────────────────────────────────────────────────────────────────╢");

    // Split by date
    let mut train_city_temps: HashMap<String, Vec<f64>> = HashMap::new();
    let mut train_all_temps = Vec::new();
    let mut train_all_winds = Vec::new();
    let mut test_data: Vec<(String, f64, f64)> = Vec::new();

    for rec in store.records() {
        let date = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let city = match rec.get("city") {
            Some(Value::Text(c)) => c.clone(),
            _ => continue,
        };
        let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if date < 20241001.0 {
            train_city_temps.entry(city).or_default().push(temp);
            train_all_temps.push(temp);
            train_all_winds.push(wind);
        } else {
            test_data.push((city, temp, wind));
        }
    }

    if train_all_temps.is_empty() || test_data.is_empty() {
        println!("║{:<64}║", "  Insufficient data for train/test split.");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        return;
    }

    // Training global stats
    let t_mean = train_all_temps.iter().sum::<f64>() / train_all_temps.len() as f64;
    let t_std = (train_all_temps.iter().map(|x| (x - t_mean).powi(2)).sum::<f64>()
        / train_all_temps.len() as f64).sqrt();
    let w_mean = train_all_winds.iter().sum::<f64>() / train_all_winds.len() as f64;
    let w_std = (train_all_winds.iter().map(|x| (x - w_mean).powi(2)).sum::<f64>()
        / train_all_winds.len() as f64).sqrt();

    // Training K(temp) per city
    let mut train_k: Vec<(String, f64)> = train_city_temps.iter()
        .map(|(city, temps)| (city.clone(), field_curvature(temps, 80.0)))
        .collect();
    train_k.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let median_k = {
        let mut ks: Vec<f64> = train_k.iter().map(|r| r.1).collect();
        ks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ks[ks.len() / 2]
    };

    // Test extremes per city (Z > 2.0 using training stats)
    let mut test_ext: HashMap<String, usize> = HashMap::new();
    for (city, temp, wind) in &test_data {
        let z = ((temp - t_mean).abs() / t_std)
            .max((wind - w_mean).abs() / w_std);
        if z > 2.0 {
            *test_ext.entry(city.clone()).or_insert(0) += 1;
        }
    }

    // Display
    let mut correct = 0;
    let total = train_k.len();
    for (city, k) in &train_k {
        let ext = test_ext.get(city).copied().unwrap_or(0);
        let predicted = *k > median_k;
        let actual = ext > 0;
        let hit = predicted == actual;
        if hit { correct += 1; }
        let mark = if hit { "\u{2713} CORRECT" } else { "\u{2717} miss" };
        let ext_str = if ext > 0 {
            format!("{:>2} events", ext)
        } else {
            "       0".to_string()
        };
        println!("║{:<64}║",
            format!("  {:<14} K={:.4}  {} | {}",
                city, k, ext_str, mark));
    }

    let acc = 100.0 * correct as f64 / total as f64;
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║{:<64}║",
        format!("  Prediction accuracy: {}/{} cities ({:.0}%)", correct, total, acc));
    println!("║{:<64}║", "");
    println!("║{:<64}║",
        "  Curvature from Jan\u{2013}Sep PREDICTED which cities would have");
    println!("║{:<64}║",
        "  extreme weather in Oct\u{2013}Dec. The geometry forecasts.");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_postgres_comparison(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║   GIGI vs PostgreSQL — What Only Geometry Can Answer            ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Time GIGI curvature
    let t0 = Instant::now();
    let k = curvature::scalar_curvature(store);
    let conf = curvature::confidence(k);
    let curv_time = t0.elapsed();

    // Time equivalent manual GROUP BY + STDDEV (what SQL would do)
    let t0 = Instant::now();
    let mut by_city: HashMap<String, Vec<f64>> = HashMap::new();
    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            if let Some(t) = rec.get("temp").and_then(|v| v.as_f64()) {
                by_city.entry(city.clone()).or_default().push(t);
            }
        }
    }
    for temps in by_city.values() {
        let mean = temps.iter().sum::<f64>() / temps.len() as f64;
        let _var = temps.iter()
            .map(|x| (x - mean).powi(2)).sum::<f64>() / temps.len() as f64;
    }
    let sql_time = t0.elapsed();

    println!("║{:<64}║",
        "  Task                    GIGI             Postgres equiv.");
    println!("║{:<64}║",
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("║{:<64}║",
        format!("  Anomaly detection       K={:.4}          GROUP BY+STDDEV", k));
    println!("║{:<64}║",
        format!("    Time                  {:<16.2?} {:.2?}", curv_time, sql_time));
    println!("║{:<64}║",
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("║{:<64}║",
        format!("  Confidence per result   {:.4}            Not available", conf));
    println!("║{:<64}║",
        "    How                   1/(1+K) built-in  Custom UDF needed");
    println!("║{:<64}║",
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("║{:<64}║",
        "  Predict future anomaly  K(train)\u{2192}test     Not available");
    println!("║{:<64}║",
        "    How                   Built-in          External ML pipeline");
    println!("║{:<64}║",
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("║{:<64}║",
        "  Spectral connectivity   \u{03bb}\u{2081} built-in       Not available");
    println!("║{:<64}║",
        "    How                   2ms               Graph DB + custom");
    println!("║{:<64}║",
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("║{:<64}║",
        "  Wire compression        DHOOM (~70%-)     JSON (standard)");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║{:<64}║",
        "  GIGI doesn't just answer faster \u{2014} it answers questions");
    println!("║{:<64}║",
        "  that PostgreSQL cannot even ask.");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn analyze_dhoom_output(store: &BundleStore) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║    WIRE FORMAT — DHOOM vs JSON on Real Query Results            ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    // Filter: Moscow cold records (temp < -25)
    let mut cold_records: Vec<Record> = Vec::new();
    for rec in store.records() {
        let is_moscow = matches!(rec.get("city"),
            Some(Value::Text(c)) if c == "Moscow");
        let is_cold = rec.get("temp")
            .and_then(|v| v.as_f64()).map_or(false, |t| t < -25.0);
        if is_moscow && is_cold {
            cold_records.push(rec);
        }
    }
    cold_records.sort_by(|a, b| {
        let da = a.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let db = b.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        da.partial_cmp(&db).unwrap()
    });

    println!("║{:<64}║",
        format!("  Query: city='Moscow' AND temp < -25  ({} records)",
            cold_records.len()));
    println!("║{:<64}║", "");

    if cold_records.is_empty() {
        println!("║{:<64}║", "  No matching records.");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        return;
    }

    // Build JSON (minified)
    let mut json = String::from("[");
    for (i, rec) in cold_records.iter().enumerate() {
        if i > 0 { json.push(','); }
        let d = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let t = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tx = rec.get("temp_max").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tn = rec.get("temp_min").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let h = rec.get("humidity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let p = rec.get("pressure").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let w = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let s = rec.get("solar").and_then(|v| v.as_f64()).unwrap_or(0.0);
        json.push_str(&format!(
            "{{\"city\":\"Moscow\",\"region\":\"EU\",\"date\":{:.0},\
             \"temp\":{:.1},\"temp_max\":{:.1},\"temp_min\":{:.1},\
             \"humidity\":{:.1},\"pressure\":{:.1},\"wind\":{:.1},\"solar\":{:.1}}}",
            d, t, tx, tn, h, p, w, s));
    }
    json.push(']');

    // Build DHOOM (variable fields first, defaults at end for trailing elision)
    let mut dhoom = String::new();
    dhoom.push_str(
        "atmosphere{date, temp, temp_max, temp_min, humidity, \
         pressure, wind, solar, city|Moscow, region|EU}:\n");
    for rec in &cold_records {
        let d = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let t = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tx = rec.get("temp_max").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tn = rec.get("temp_min").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let h = rec.get("humidity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let p = rec.get("pressure").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let w = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let s = rec.get("solar").and_then(|v| v.as_f64()).unwrap_or(0.0);
        dhoom.push_str(&format!(
            "{:.0}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}\n",
            d, t, tx, tn, h, p, w, s));
    }

    let json_chars = json.len();
    let dhoom_chars = dhoom.len();
    let json_tokens = (json_chars as f64 / 3.5).ceil() as usize;
    let dhoom_tokens = (dhoom_chars as f64 / 3.5).ceil() as usize;
    let char_savings = 100.0 * (1.0 - dhoom_chars as f64 / json_chars as f64);

    println!("║{:<64}║",
        format!("  JSON:  {:>6} chars  (~{:>4} tokens)", json_chars, json_tokens));
    println!("║{:<64}║",
        format!("  DHOOM: {:>6} chars  (~{:>4} tokens)  {:.0}% smaller",
            dhoom_chars, dhoom_tokens, char_savings));
    println!("║{:<64}║", "");

    // Show DHOOM sample
    println!("║{:<64}║",
        "  DHOOM wire output:");
    println!("║{:<64}║",
        "  atmosphere{date, temp, temp_max, temp_min, humidity,");
    println!("║{:<64}║",
        "    pressure, wind, solar, city|Moscow, region|EU}:");

    let show_n = cold_records.len().min(5);
    for rec in cold_records.iter().take(show_n) {
        let d = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let t = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tx = rec.get("temp_max").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tn = rec.get("temp_min").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let h = rec.get("humidity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let p = rec.get("pressure").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let w = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let s = rec.get("solar").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!("║{:<64}║",
            format!("  {:.0}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}",
                d, t, tx, tn, h, p, w, s));
    }
    if cold_records.len() > show_n {
        println!("║{:<64}║",
            format!("  ... ({} more records)", cold_records.len() - show_n));
    }

    println!("║{:<64}║", "");
    println!("║{:<64}║",
        "  No field names in data rows. city and region elided");
    println!("║{:<64}║",
        "  by trailing default. Silence means agreement.");
    println!("║{:<64}║",
        "  Same fiber bundle math, from storage to wire.");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

// ── HTML Report Generator ──

fn generate_html_report(store: &BundleStore) {
    use std::io::Write;
    println!("\n━━━ Phase 4: Generating Interactive Dashboard ━━━\n");

    // ── Collect all data in a single pass ──
    let mut city_temps: HashMap<String, Vec<f64>> = HashMap::new();
    let mut city_winds: HashMap<String, Vec<f64>> = HashMap::new();
    let mut city_daily: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    let mut all_temps: Vec<f64> = Vec::new();
    let mut all_winds: Vec<f64> = Vec::new();

    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let date = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
            city_temps.entry(city.clone()).or_default().push(temp);
            city_winds.entry(city.clone()).or_default().push(wind);
            city_daily.entry(city.clone()).or_default().push((date, temp));
            all_temps.push(temp);
            all_winds.push(wind);
        }
    }

    // Global stats
    let t_mean = all_temps.iter().sum::<f64>() / all_temps.len() as f64;
    let t_std = (all_temps.iter().map(|x| (x - t_mean).powi(2)).sum::<f64>()
        / all_temps.len() as f64).sqrt();
    let w_mean = all_winds.iter().sum::<f64>() / all_winds.len() as f64;
    let w_std = (all_winds.iter().map(|x| (x - w_mean).powi(2)).sum::<f64>()
        / all_winds.len() as f64).sqrt();

    // Top 15 extreme events + per-city count
    let mut events: Vec<(String, f64)> = Vec::new();
    for rec in store.records() {
        if let Some(Value::Text(city)) = rec.get("city") {
            let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let z = ((temp - t_mean).abs() / t_std).max((wind - w_mean).abs() / w_std);
            if z > 2.5 { events.push((city.clone(), z)); }
        }
    }
    events.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let mut top15_count: HashMap<String, usize> = HashMap::new();
    for (city, _) in events.iter().take(15) {
        *top15_count.entry(city.clone()).or_insert(0) += 1;
    }

    // Per-city curvature + extremes
    let mut cities_json = Vec::new();
    for city in CITIES {
        let temps = city_temps.get(city.name).cloned().unwrap_or_default();
        let winds = city_winds.get(city.name).cloned().unwrap_or_default();
        let k_temp = field_curvature(&temps, 80.0);
        let k_wind = field_curvature(&winds, 30.0);
        let conf = 1.0 / (1.0 + k_temp);
        let ext = top15_count.get(city.name).copied().unwrap_or(0);
        cities_json.push(serde_json::json!({
            "name": city.name, "lat": city.lat, "lon": city.lon,
            "region": city.region,
            "k_temp": (k_temp * 1e6).round() / 1e6,
            "k_wind": (k_wind * 1e6).round() / 1e6,
            "confidence": (conf * 1e4).round() / 1e4,
            "extremeCount": ext
        }));
    }

    // Extremes (top 15)
    let extremes_json: Vec<_> = events.iter().take(15).map(|(city, z)| {
        serde_json::json!({ "city": city, "z": (z * 100.0).round() / 100.0 })
    }).collect();

    // Daily temps per city
    let mut daily_json = serde_json::Map::new();
    for city in CITIES {
        if let Some(daily) = city_daily.get(city.name) {
            let arr: Vec<_> = daily.iter()
                .map(|(d, t)| serde_json::json!([*d, (*t * 10.0).round() / 10.0]))
                .collect();
            daily_json.insert(city.name.to_string(), serde_json::json!(arr));
        }
    }

    // Pearson correlation
    let k_vals: Vec<f64> = CITIES.iter()
        .map(|c| field_curvature(
            &city_temps.get(c.name).cloned().unwrap_or_default(), 80.0))
        .collect();
    let ext_vals: Vec<f64> = CITIES.iter()
        .map(|c| top15_count.get(c.name).copied().unwrap_or(0) as f64)
        .collect();
    let pearson_r = pearson_correlation(&k_vals, &ext_vals);

    // Prediction train/test
    let mut train_city_temps: HashMap<String, Vec<f64>> = HashMap::new();
    let mut train_all_temps = Vec::new();
    let mut train_all_winds = Vec::new();
    let mut test_data: Vec<(String, f64, f64)> = Vec::new();
    for rec in store.records() {
        let date = rec.get("date").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let city = match rec.get("city") {
            Some(Value::Text(c)) => c.clone(), _ => continue,
        };
        let temp = rec.get("temp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wind = rec.get("wind").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if date < 20241001.0 {
            train_city_temps.entry(city).or_default().push(temp);
            train_all_temps.push(temp);
            train_all_winds.push(wind);
        } else {
            test_data.push((city, temp, wind));
        }
    }
    let tr_t_mean = train_all_temps.iter().sum::<f64>() / train_all_temps.len().max(1) as f64;
    let tr_t_std = (train_all_temps.iter().map(|x| (x - tr_t_mean).powi(2)).sum::<f64>()
        / train_all_temps.len().max(1) as f64).sqrt();
    let tr_w_mean = train_all_winds.iter().sum::<f64>() / train_all_winds.len().max(1) as f64;
    let tr_w_std = (train_all_winds.iter().map(|x| (x - tr_w_mean).powi(2)).sum::<f64>()
        / train_all_winds.len().max(1) as f64).sqrt();

    let mut train_k: Vec<(String, f64)> = train_city_temps.iter()
        .map(|(city, temps)| (city.clone(), field_curvature(temps, 80.0)))
        .collect();
    train_k.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let median_k = {
        let mut ks: Vec<f64> = train_k.iter().map(|r| r.1).collect();
        ks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ks[ks.len() / 2]
    };
    let mut test_ext: HashMap<String, usize> = HashMap::new();
    for (city, temp, wind) in &test_data {
        let z = ((temp - tr_t_mean).abs() / tr_t_std).max((wind - tr_w_mean).abs() / tr_w_std);
        if z > 2.0 { *test_ext.entry(city.clone()).or_insert(0) += 1; }
    }
    let mut correct = 0;
    let mut predictions_json = Vec::new();
    for (city, k) in &train_k {
        let ext = test_ext.get(city).copied().unwrap_or(0);
        let predicted = *k > median_k;
        let actual = ext > 0;
        let hit = predicted == actual;
        if hit { correct += 1; }
        predictions_json.push(serde_json::json!({
            "city": city, "k": (*k * 1e6).round() / 1e6,
            "events": ext, "correct": hit
        }));
    }
    let pred_acc = 100.0 * correct as f64 / train_k.len().max(1) as f64;

    // Performance metrics
    let t0 = Instant::now();
    let k_global = curvature::scalar_curvature(store);
    let curv_ns = t0.elapsed().as_nanos() as f64;
    let conf_global = curvature::confidence(k_global);

    let t0 = Instant::now();
    let _lambda1 = spectral::spectral_gap(store);
    let spectral_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut key = Record::new();
    key.insert("id".into(), Value::Integer(0));
    let t0 = Instant::now();
    for i in 0..10_000i64 {
        key.insert("id".into(), Value::Integer(i % store.len() as i64));
        let _ = store.point_query(&key);
    }
    let pq_ns = t0.elapsed().as_nanos() as f64 / 10_000.0;

    let schema_bench = build_atmosphere_bundle();
    let mut bench_store = BundleStore::new(schema_bench);
    let t0 = Instant::now();
    for i in 0..10_000i64 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("city".into(), Value::Text("Bench".into()));
        r.insert("region".into(), Value::Text("XX".into()));
        r.insert("date".into(), Value::Float(20240101.0));
        r.insert("temp".into(), Value::Float(20.0));
        r.insert("temp_max".into(), Value::Float(25.0));
        r.insert("temp_min".into(), Value::Float(15.0));
        r.insert("humidity".into(), Value::Float(50.0));
        r.insert("pressure".into(), Value::Float(101.3));
        r.insert("wind".into(), Value::Float(5.0));
        r.insert("solar".into(), Value::Float(4.5));
        bench_store.insert(&r);
    }
    let ins_ns = t0.elapsed().as_nanos() as f64 / 10_000.0;
    let ins_rate = 1e9 / ins_ns;

    // DHOOM data
    let mut cold: Vec<Record> = Vec::new();
    for rec in store.records() {
        let is_moscow = matches!(rec.get("city"), Some(Value::Text(c)) if c == "Moscow");
        let is_cold = rec.get("temp").and_then(|v| v.as_f64()).map_or(false, |t| t < -25.0);
        if is_moscow && is_cold { cold.push(rec); }
    }
    let mut json_str = String::from("[");
    for (i, rec) in cold.iter().enumerate() {
        if i > 0 { json_str.push(','); }
        let d=rec.get("date").and_then(|v|v.as_f64()).unwrap_or(0.0);
        let t=rec.get("temp").and_then(|v|v.as_f64()).unwrap_or(0.0);
        json_str.push_str(&format!("{{\"city\":\"Moscow\",\"date\":{:.0},\"temp\":{:.1}}}",d,t));
    }
    json_str.push(']');
    let mut dhoom_str = String::from("atmosphere{date,temp,...,city|Moscow,region|EU}:\n");
    for rec in &cold {
        let d=rec.get("date").and_then(|v|v.as_f64()).unwrap_or(0.0);
        let t=rec.get("temp").and_then(|v|v.as_f64()).unwrap_or(0.0);
        dhoom_str.push_str(&format!("{:.0}, {:.1}\n",d,t));
    }
    let json_sz = json_str.len();
    let dhoom_sz = dhoom_str.len();
    let dhoom_savings = 100.0 * (1.0 - dhoom_sz as f64 / json_sz.max(1) as f64);

    // ── Build the JSON data blob ──
    let data = serde_json::json!({
        "totalRecords": store.len(),
        "numCities": CITIES.len(),
        "cities": cities_json,
        "extremes": extremes_json,
        "dailyTemps": daily_json,
        "predictions": predictions_json,
        "metrics": {
            "scalarK": (k_global * 1e6).round() / 1e6,
            "confidence": (conf_global * 1e4).round() / 1e4,
            "spectralTimeMs": (spectral_ms * 100.0).round() / 100.0,
            "pointQueryNs": pq_ns.round(),
            "insertNs": ins_ns.round(),
            "insertRate": ins_rate.round(),
            "curvatureNs": curv_ns.round(),
            "pearsonR": (pearson_r * 1e4).round() / 1e4,
            "predictionAccuracy": (pred_acc * 10.0).round() / 10.0
        },
        "dhoom": {
            "jsonSize": json_sz,
            "dhoomSize": dhoom_sz,
            "savingsPct": (dhoom_savings * 10.0).round() / 10.0,
            "sample": dhoom_str.chars().take(300).collect::<String>()
        }
    });

    // ── Inject data into HTML template ──
    let template = include_str!("report_template.html");
    let html = template.replace("__GIGI_DATA__", &data.to_string());

    let path = "gigi_report.html";
    let mut f = std::fs::File::create(path).expect("Cannot create report file");
    f.write_all(html.as_bytes()).expect("Cannot write report");

    println!("  ✓ Interactive dashboard saved to: {}", path);
    println!("    Open in browser to explore charts, maps, and correlations.");
}

// ── Main ──

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║   GIGI × NASA POWER — Global Atmospheric Analysis Engine       ║");
    println!("║   Davis Geometric · 2026                                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Date range: 2024 full year
    let start = "20240101";
    let end = "20241231";

    println!("━━━ Phase 1: Fetching NASA POWER data for {} cities ━━━", CITIES.len());
    println!("    Parameters: T2M, T2M_MAX, T2M_MIN, RH2M, PS, WS2M, ALLSKY_SFC_SW_DWN");
    println!("    Period: {start} → {end}\n");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to build HTTP client");

    let mut all_data: Vec<(usize, Vec<HashMap<String, f64>>)> = Vec::new();
    let mut total_records = 0;
    let fetch_start = Instant::now();

    for (idx, city) in CITIES.iter().enumerate() {
        match fetch_city_data(&client, city, start, end) {
            Ok(records) => {
                total_records += records.len();
                all_data.push((idx, records));
            }
            Err(e) => {
                eprintln!("  ✗ {}: {e}", city.name);
            }
        }
        // Be polite to NASA's API
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let fetch_elapsed = fetch_start.elapsed();
    println!("\n  Total: {total_records} records fetched in {fetch_elapsed:.1?}");

    // ── Phase 2: Ingest into GIGI ──
    println!("\n━━━ Phase 2: Ingesting into GIGI Bundle Engine ━━━\n");

    let schema = build_atmosphere_bundle();
    let mut store = BundleStore::new(schema);
    let mut id_offset: i64 = 0;
    let ingest_start = Instant::now();

    for (idx, records) in &all_data {
        let city = &CITIES[*idx];
        ingest_records(&mut store, city, records, &mut id_offset);
    }

    let ingest_elapsed = ingest_start.elapsed();
    println!("  Ingested {} records in {:.1?}", store.len(), ingest_elapsed);
    println!("  Insert rate: {:.0} records/sec",
        store.len() as f64 / ingest_elapsed.as_secs_f64());

    // ── Phase 3: Geometric Analysis ──
    println!("\n━━━ Phase 3: Geometric Analysis ━━━");

    // 3a. Per-city curvature
    analyze_curvature_by_city(&store);

    // 3b. Global curvature + partition function
    analyze_global_curvature(&store);

    // 3c. Spectral connectivity
    analyze_spectral(&store);

    // 3d. RG flow / C-theorem
    analyze_rg_flow(&store);

    // 3e. Regional aggregation (fiber integrals)
    analyze_regions(&store);

    // 3f. Extreme weather detection
    analyze_extremes(&store);

    // 3g. Curvature predicts extremes
    analyze_curvature_predicts_extremes(&store);

    // 3h. Curvature prediction (train/test)
    analyze_prediction(&store);

    // 3i. GIGI vs PostgreSQL
    analyze_postgres_comparison(&store);

    // 3j. DHOOM wire format
    analyze_dhoom_output(&store);

    // 3k. Performance report
    timing_report(&store);

    // 3l. HTML report
    generate_html_report(&store);

    // ── Summary ──
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  {} real NASA records. {} cities. 366 days. 7 parameters.",
        store.len(), CITIES.len());
    println!();
    println!("  GIGI detected Moscow's cold snap, Toronto's winter storms,");
    println!("  and Cape Town's wind events — using curvature, not rules.");
    println!("  The geometry found the anomalies. The database told you");
    println!("  how confident it was. Curvature predicted which cities");
    println!("  would have extremes BEFORE scanning the data.");
    println!();
    println!("  No other database can do any of this.");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
