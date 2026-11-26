//! The registered control runs, read from their receipts (docs/research/helike-view-controls-scene-spec-2026-11-26.md).
//!
//! Every suite has an adapter that maps its report into one model: a suite is a set of control points (tiles), a tile
//! is a set of checks, and a check has a value, a tolerance and an outcome. Every value an adapter keeps is copied out
//! of the report and remembers the JSON pointer it came from, so a test can compare the two bit for bit. Outcomes are
//! read from the report's own verdict strings; nothing here recomputes a gate, a seam or a verdict. The only judgement
//! made here is the prediction mark, and that only compares a report value with the condition its registration
//! wrote down in `predictions.json` (exact equality, an inequality, or agreement to the quoted significant figures).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The control points every suite lines up under: g² × Mq, then a column for checks across the pair of couplings.
pub const COLUMNS: [&str; 5] = [
    "g² 0.002 · Mq 8",
    "g² 0.002 · Mq 10",
    "g² 0.004 · Mq 8",
    "g² 0.004 · Mq 10",
    "across the pair",
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    Open,
    /// A value the registration reports but never decides on (drawn grey whatever it is).
    Reported,
    /// The check does not apply at this point (e.g. S3 at Mq 8, where the ladder has no next step).
    NotApplicable,
    Queued,
    Running,
}

impl Outcome {
    /// A verdict string from a report. Anything not recognised is OPEN: an unknown word is never drawn green.
    pub fn from_word(s: &str) -> Outcome {
        match s {
            "PASS" | "RESOLVED" | "CORRIDOR" | "GLUING-INVARIANT" | "AGREES"
            | "SIGNAL-INVARIANT" | "TONE-STABLE" | "ALIGNED" => Outcome::Pass,
            "FAIL" | "SPLIT" | "DISAGREES" | "GLUING-SENSITIVE" | "SIGNAL-SENSITIVE"
            | "TONE-SHIFT" => Outcome::Fail,
            _ => Outcome::Open,
        }
    }
    fn from_holds(v: &Value) -> Outcome {
        match v.as_bool() {
            Some(true) => Outcome::Pass,
            Some(false) => Outcome::Fail,
            None => Outcome::NotApplicable,
        }
    }
    /// For a tile: the worst decided outcome among its checks (FAIL over OPEN over PASS); reported-only checks and
    /// checks that do not apply never colour it.
    pub fn worst(it: impl IntoIterator<Item = Outcome>) -> Outcome {
        let mut w = None;
        for o in it {
            let rank = match o {
                Outcome::Fail => 3,
                Outcome::Open => 2,
                Outcome::Pass => 1,
                _ => continue,
            };
            if w.map_or(true, |(r, _)| rank > r) {
                w = Some((rank, o));
            }
        }
        w.map_or(Outcome::Open, |(_, o)| o)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    /// e.g. "e_cut", "S2 quadrature"
    pub name: String,
    /// the state or sector it belongs to, if any
    pub group: Option<String>,
    pub value: Option<f64>,
    pub tolerance: Option<f64>,
    pub outcome: Outcome,
    pub reported_only: bool,
    /// a short text form when the check is not a number (clusters, radii, a word)
    pub text: Option<String>,
    /// where `value` came from in the report (RFC 6901 pointer), for the bit-for-bit test
    pub pointer: Option<String>,
}

impl Check {
    fn new(name: &str, group: Option<&str>) -> Check {
        Check {
            name: name.into(),
            group: group.map(Into::into),
            value: None,
            tolerance: None,
            outcome: Outcome::Open,
            reported_only: false,
            text: None,
            pointer: None,
        }
    }
    fn number(mut self, report: &Value, pointer: &str) -> Check {
        self.value = report.pointer(pointer).and_then(Value::as_f64);
        self.pointer = Some(pointer.into());
        self
    }
    fn tol(mut self, t: f64) -> Check {
        self.tolerance = Some(t);
        self
    }
    fn outcome(mut self, o: Outcome) -> Check {
        self.outcome = o;
        self
    }
    fn reported(mut self) -> Check {
        self.reported_only = true;
        self.outcome = Outcome::Reported;
        self
    }
    fn text(mut self, t: impl Into<String>) -> Check {
        self.text = Some(t.into());
        self
    }
}

/// A table in the tile detail. Cells carry an outcome only where the report decided one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    pub title: String,
    pub columns: Vec<String>,
    /// columns holding reported-only values (drawn grey, headed "reported, not decided")
    pub reported_columns: Vec<usize>,
    pub rows: Vec<Vec<Cell>>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub text: String,
    pub outcome: Option<Outcome>,
}

fn cell(text: impl Into<String>) -> Cell {
    Cell {
        text: text.into(),
        outcome: None,
    }
}
fn cell_o(text: impl Into<String>, o: Outcome) -> Cell {
    Cell {
        text: text.into(),
        outcome: Some(o),
    }
}

/// One heat-signal sector at one coupling: each source with its plateau energy and missing weight.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeatSector {
    pub sector: String,
    pub verdict: Outcome,
    pub corridor_level: Option<i64>,
    /// (source, plateau energy, missing weight)
    pub sources: Vec<(String, f64, f64)>,
}

/// One M8 channel at one control: the shares at both seam orders, the raw signed block matrix per level, and the
/// shape price against the level. Every number is copied from the report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct M8Channel {
    pub name: String,
    pub block: String,
    pub label: String,
    /// (order, shares H C S R) for the coarse and the fine order of the seam pair
    pub shares: Vec<(i64, [f64; 4])>,
    /// per level: the 4x4 raw D_XY / D, signed
    pub raw: Vec<[[f64; 4]; 4]>,
    pub shape_price: Option<f64>,
    pub gap: Option<f64>,
    pub discount: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct M8Detail {
    pub channels: Vec<M8Channel>,
    /// the report's own ordering record: rank of Helike by shape and by level
    pub ordering_shapes: Option<i64>,
    pub ordering_levels: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub stage: String,
    /// unix seconds of the last event
    pub t: Option<f64>,
    pub done: bool,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    /// first column and how many columns it spans
    pub col: usize,
    pub span: usize,
    pub outcome: Outcome,
    /// the report's own word for the outcome, shown on the tile ("PASS", "GLUING-INVARIANT", ...)
    pub word: String,
    pub checks: Vec<Check>,
    pub commit: Option<String>,
    pub tag: Option<String>,
    pub tables: Vec<Table>,
    pub heat: Vec<HeatSector>,
    /// M8's geometry for the tile detail (spec section 4), when the tile is an M8 control
    #[serde(default)]
    pub m8: Option<M8Detail>,
    pub progress: Option<Progress>,
    /// set when a later amendment replaced this tile; it is kept and drawn struck through
    pub superseded_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prediction {
    pub quote: String,
    pub source: String,
    pub column: Option<usize>,
    pub outcome_text: String,
    pub mark: Mark,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Mark {
    Met,
    Missed,
    NotYetRun,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suite {
    pub key: String,
    pub name: String,
    pub registration: String,
    /// the report file, relative to the root
    pub source: String,
    pub tiles: Vec<Tile>,
    /// tiles a later amendment replaced; kept
    pub superseded: Vec<Tile>,
    pub verdict: String,
    pub verdict_outcome: Outcome,
    /// shown in amber when the receipt is missing or does not match its test receipts
    pub warning: Option<String>,
    pub predictions: Vec<Prediction>,
    /// true for a suite whose values are interval enclosures (R2)
    pub interval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub date: String,
    pub title: String,
    pub doc: String,
    /// suite keys whose tiles this amendment changed
    #[serde(default)]
    pub changed: Vec<String>,
    /// tiles it superseded: the old report and the column
    #[serde(default)]
    pub supersedes: Vec<Superseded>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Superseded {
    pub suite: String,
    pub column: usize,
    /// the archived report, relative to the root, and the pointer to its verdict word at that point
    pub report: String,
    pub verdict_pointer: String,
    pub note: String,
}

// ------------------------------------------------------------------------------------------------ loading

pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn domain(&self) -> PathBuf {
        self.root
            .join("research/flux-grid-20261126/gate57-controls")
    }
    pub fn gluing(&self) -> PathBuf {
        self.root
            .join("research/flux-grid-20261126/gluing-controls-gate57")
    }
    pub fn heat(&self) -> PathBuf {
        self.root.join("research/heat-signal-20261126/controls")
    }
    pub fn m8(&self) -> PathBuf {
        self.root.join("research/m8-20261126")
    }
    pub fn r2(&self) -> PathBuf {
        self.root.join("research/sc-r2-20261126")
    }
    /// The geometry export of the control runs (scripts/export_helike_geometry.py).
    pub fn geometry(&self) -> PathBuf {
        self.root.join("research/geometry-controls-20261126/geometry.json")
    }
    pub fn history(&self) -> PathBuf {
        self.root.join("research/controls-history.json")
    }
    /// Every file the scene reads; their sizes and mtimes decide when to rescan.
    pub fn watched(&self) -> Vec<PathBuf> {
        let mut v = vec![self.history(), self.geometry()];
        for d in [self.domain(), self.gluing(), self.heat()] {
            v.extend([
                d.join("report.json"),
                d.join("progress.jsonl"),
                d.join("predictions.json"),
            ]);
        }
        v.extend([
            self.m8().join("predictions.json"),
            self.r2().join("predictions.json"),
        ]);
        for d in m8_dirs(&self.m8()) {
            v.extend([d.join("report.json"), d.join("progress.jsonl")]);
        }
        v.extend(r2_files(&self.r2()));
        v
    }
}

/// Walk up from `start` to the first directory holding `research/` and `docs/research/`.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut p = Some(start);
    while let Some(d) = p {
        if d.join("research").is_dir() && d.join("docs/research").is_dir() {
            return Some(d.to_path_buf());
        }
        p = d.parent();
    }
    None
}

pub fn fingerprint(paths: &Paths) -> Vec<(PathBuf, u64, u128)> {
    paths
        .watched()
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?;
            let t = m
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some((p, m.len(), t))
        })
        .collect()
}

fn read_json(p: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn m8_dirs(m8: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(m8)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("controls"))
        })
        .collect();
    v.sort();
    v
}

fn r2_files(r2: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(r2)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("r2_g") && (n.ends_with(".txt") || n.ends_with(".json"))
            })
        })
        .collect();
    v.sort();
    v
}

/// The column of a control point.
pub fn column(g2: f64, mq: Option<i64>) -> Option<usize> {
    let g = if (g2 - 0.002).abs() < 1e-12 {
        0
    } else if (g2 - 0.004).abs() < 1e-12 {
        1
    } else {
        return None;
    };
    match mq {
        Some(8) => Some(2 * g),
        Some(10) => Some(2 * g + 1),
        None => Some(2 * g),
        _ => None,
    }
}

/// Everything, in board order.
pub fn load_all(paths: &Paths) -> (Vec<Suite>, Vec<HistoryEntry>) {
    let history: Vec<HistoryEntry> = read_json(&paths.history())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let mut suites = vec![
        load_domain(paths),
        load_gluing(paths),
        load_heat(paths),
        load_m8(paths),
        load_r2(paths),
    ];
    for s in &mut suites {
        // superseded tiles, kept and struck through
        for h in &history {
            for sup in h.supersedes.iter().filter(|x| x.suite == s.key) {
                let old = read_json(&paths.root.join(&sup.report));
                let word = old
                    .as_ref()
                    .and_then(|o| o.pointer(&sup.verdict_pointer))
                    .and_then(Value::as_str)
                    .unwrap_or("receipt missing")
                    .to_string();
                s.superseded.push(Tile {
                    col: sup.column,
                    span: 1,
                    outcome: Outcome::from_word(&word),
                    word,
                    checks: vec![],
                    commit: None,
                    tag: Some(sup.note.clone()),
                    tables: vec![],
                    heat: vec![],
                    m8: None,
                    progress: None,
                    superseded_by: Some(h.id.clone()),
                });
            }
        }
    }
    (suites, history)
}

// ------------------------------------------------------------------------------------------------ progress

/// The latest progress event per column from `progress.jsonl` (one JSON object per line; see spec §5).
pub fn read_progress(dir: &Path) -> Vec<(usize, Progress)> {
    let Ok(text) = std::fs::read_to_string(dir.join("progress.jsonl")) else {
        return vec![];
    };
    let mut out: Vec<(usize, Progress)> = vec![];
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(g2) = v.get("g2").and_then(Value::as_f64) else {
            continue;
        };
        let Some(col) = column(g2, v.get("Mq").and_then(Value::as_i64)) else {
            continue;
        };
        let stage = v
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let p = Progress {
            done: stage == "done" || v.get("done").and_then(Value::as_bool).unwrap_or(false),
            stage,
            t: v.get("t").and_then(Value::as_f64),
            commit: v.get("commit").and_then(Value::as_str).map(Into::into),
        };
        match out.iter_mut().find(|(c, _)| *c == col) {
            Some(slot) => slot.1 = p,
            None => out.push((col, p)),
        }
    }
    out
}

/// Tiles for columns that have progress but no result yet: running (or finished, waiting for the report to land).
fn merge_progress(tiles: &mut Vec<Tile>, prog: Vec<(usize, Progress)>) {
    for (col, p) in prog {
        if let Some(t) = tiles
            .iter_mut()
            .find(|t| col >= t.col && col < t.col + t.span)
        {
            t.progress = Some(p);
            continue;
        }
        tiles.push(Tile {
            col,
            span: 1,
            outcome: if p.done {
                Outcome::Queued
            } else {
                Outcome::Running
            },
            word: if p.done {
                "finished · report pending".into()
            } else {
                "running".into()
            },
            checks: vec![],
            commit: p.commit.clone(),
            tag: None,
            tables: vec![],
            heat: vec![],
            m8: None,
            progress: Some(p),
            superseded_by: None,
        });
    }
    tiles.sort_by_key(|t| t.col);
}

// ------------------------------------------------------------------------------------------------ predictions

#[derive(Deserialize)]
struct PredictionsFile {
    predictions: Vec<PredictionSpec>,
}

#[derive(Deserialize)]
struct PredictionSpec {
    quote: String,
    source: String,
    #[serde(default)]
    column: Option<usize>,
    /// None: the condition is not machine-readable; the mark stays "not yet run" until a report exists, then the
    /// hand-written `outcome` (if any) is shown.
    #[serde(default)]
    check: Option<PredCheck>,
    #[serde(default)]
    outcome: Option<String>,
    /// "met" or "missed", written by hand from the registration's own results section; only used without `check`
    #[serde(default)]
    mark: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum PredCheck {
    /// every pointer equals the value
    Equals {
        pointers: Vec<String>,
        value: Value,
    },
    /// the value at the pointer, rounded to the significant figures of `value`, equals `value` ("≈ 5.6·10⁻⁶")
    RoundsTo {
        pointer: String,
        value: f64,
        sig: u32,
    },
    Le {
        pointers: Vec<String>,
        value: f64,
    },
    Ge {
        pointers: Vec<String>,
        value: f64,
    },
}

fn round_sig(x: f64, sig: u32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let e = x.abs().log10().floor() as i32 - (sig as i32 - 1);
    let s = 10f64.powi(e);
    (x / s).round() * s
}

fn fmt_val(v: &Value) -> String {
    match v {
        Value::Number(n) => n.as_f64().map(sci).unwrap_or_else(|| n.to_string()),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn sci(x: f64) -> String {
    if x == 0.0 {
        "0".into()
    } else if x.abs() >= 1e-2 && x.abs() < 1e4 {
        let s = format!("{x:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("{x:.2e}")
    }
}

fn load_predictions(dir: &Path, report: Option<&Value>) -> Vec<Prediction> {
    let Some(file) = read_json(&dir.join("predictions.json"))
        .and_then(|v| serde_json::from_value::<PredictionsFile>(v).ok())
    else {
        return vec![];
    };
    file.predictions
        .into_iter()
        .map(|p| {
            let (mark, outcome_text) = match (report, &p.check) {
                (None, _) => (Mark::NotYetRun, "not yet run".to_string()),
                (Some(_), None) => match (&p.outcome, p.mark.as_deref()) {
                    (Some(o), Some("met")) => (Mark::Met, o.clone()),
                    (Some(o), Some("missed")) => (Mark::Missed, o.clone()),
                    (Some(o), _) => (Mark::NotYetRun, o.clone()),
                    (None, _) => (Mark::NotYetRun, "outcome not yet recorded".to_string()),
                },
                (Some(r), Some(c)) => evaluate(r, c),
            };
            Prediction {
                quote: p.quote,
                source: p.source,
                column: p.column,
                outcome_text,
                mark,
            }
        })
        .collect()
}

fn evaluate(r: &Value, c: &PredCheck) -> (Mark, String) {
    let missing = || (Mark::NotYetRun, "value not in the report".to_string());
    match c {
        PredCheck::Equals { pointers, value } => {
            let got: Option<Vec<&Value>> = pointers.iter().map(|p| r.pointer(p)).collect();
            let Some(got) = got else { return missing() };
            let ok = got.iter().all(|g| *g == value);
            let shown: Vec<String> = got.iter().map(|g| fmt_val(g)).collect();
            (if ok { Mark::Met } else { Mark::Missed }, shown.join(", "))
        }
        PredCheck::RoundsTo {
            pointer,
            value,
            sig,
        } => {
            let Some(x) = r.pointer(pointer).and_then(Value::as_f64) else {
                return missing();
            };
            let ok = round_sig(x, *sig) == round_sig(*value, *sig);
            (if ok { Mark::Met } else { Mark::Missed }, sci(x))
        }
        PredCheck::Le { pointers, value } | PredCheck::Ge { pointers, value } => {
            let le = matches!(c, PredCheck::Le { .. });
            let got: Option<Vec<f64>> = pointers
                .iter()
                .map(|p| r.pointer(p).and_then(Value::as_f64))
                .collect();
            let Some(got) = got else { return missing() };
            let ok = got
                .iter()
                .all(|x| if le { *x <= *value } else { *x >= *value });
            let worst = if le {
                got.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            } else {
                got.iter().cloned().fold(f64::INFINITY, f64::min)
            };
            (
                if ok { Mark::Met } else { Mark::Missed },
                format!("{} {}", if le { "max" } else { "min" }, sci(worst)),
            )
        }
    }
}

// ------------------------------------------------------------------------------------------------ adapters

fn missing_suite(
    key: &str,
    name: &str,
    registration: &str,
    source: String,
    warning: &str,
    predictions: Vec<Prediction>,
    interval: bool,
) -> Suite {
    Suite {
        key: key.into(),
        name: name.into(),
        registration: registration.into(),
        source,
        tiles: vec![],
        superseded: vec![],
        verdict: "no receipt".into(),
        verdict_outcome: Outcome::Open,
        warning: Some(warning.into()),
        predictions,
        interval,
    }
}

/// A report that names a commit, set beside the suite's test receipts: amber if they differ.
fn commit_warning(dir: &Path, report_commit: Option<&str>) -> Option<String> {
    let rc = report_commit?;
    let tests = std::fs::read_to_string(dir.join("receipts/code_parent_commit.txt")).ok()?;
    let tc = tests.trim();
    let same = tc.starts_with(rc) || rc.starts_with(tc);
    (!same).then(|| format!("report from commit {rc}, test receipts from {tc}"))
}

pub fn load_domain(paths: &Paths) -> Suite {
    let dir = paths.domain();
    let src = rel(&paths.root, &dir.join("report.json"));
    let reg = "flux-method amendment §5.7";
    let name = "Domain gate";
    let Some(r) = read_json(&dir.join("report.json")) else {
        let mut s = missing_suite(
            "domain",
            name,
            reg,
            src,
            "report.json is missing",
            load_predictions(&dir, None),
            false,
        );
        merge_progress(&mut s.tiles, read_progress(&dir));
        return s;
    };
    let commit = r.get("commit").and_then(Value::as_str).map(String::from);
    let mut tiles = vec![];
    for (i, p) in r
        .get("points")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let (Some(g2), mq) = (
            p.get("g2").and_then(Value::as_f64),
            p.get("Mq").and_then(Value::as_i64),
        ) else {
            continue;
        };
        let Some(col) = column(g2, mq) else { continue };
        let word = p
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("OPEN")
            .to_string();
        let mut checks = vec![];
        let mut rows = vec![];
        for st in ["helike_plus", "helike_minus", "competitor"] {
            let base = format!("/points/{i}/states/{st}");
            let s = p.pointer(&format!("/states/{st}"));
            let gate = s
                .and_then(|s| s.get("cut_gate"))
                .and_then(Value::as_str)
                .unwrap_or("OPEN");
            let e = Check::new("e_cut", Some(st))
                .number(&r, &format!("{base}/cut_energy_upper"))
                .tol(3e-4)
                .outcome(Outcome::from_word(gate));
            let tail = s
                .and_then(|s| s.get("tail_budget_passed"))
                .cloned()
                .unwrap_or(Value::Null);
            let norm = s
                .and_then(|s| s.get("norm_width_budget_passed"))
                .cloned()
                .unwrap_or(Value::Null);
            let ratio = Check::new("ℓ/r", Some(st))
                .number(&r, &format!("{base}/relative_ratio_reported"))
                .tol(1e-3)
                .reported();
            let ru = Check::new("r upper", Some(st))
                .number(&r, &format!("{base}/r_upper"))
                .reported();
            rows.push(vec![
                cell(st.replace('_', " ")),
                cell_o(e.value.map(sci).unwrap_or("—".into()), e.outcome),
                cell_o(word_of_bool(&tail), Outcome::from_holds(&tail)),
                cell_o(word_of_bool(&norm), Outcome::from_holds(&norm)),
                cell(ratio.value.map(sci).unwrap_or("—".into())),
                cell(ru.value.map(sci).unwrap_or("—".into())),
            ]);
            checks.push(e);
            checks.push(
                Check::new("tail² budget", Some(st))
                    .outcome(Outcome::from_holds(&tail))
                    .text(word_of_bool(&tail)),
            );
            checks.push(
                Check::new("norm width", Some(st))
                    .outcome(Outcome::from_holds(&norm))
                    .text(word_of_bool(&norm)),
            );
            checks.push(ratio);
            checks.push(ru);
        }
        let table = Table {
            title: "the three gated states".into(),
            columns: ["state", "e_cut upper (≤ 3·10⁻⁴)", "tail² ≤ 10⁻¹²", "norm width ≤ 10⁻¹⁰", "ℓ/r", "r upper"].map(String::from).to_vec(),
            reported_columns: vec![4, 5],
            rows,
            note: Some("The receipt records whether the tail and normalisation budgets passed, not their values. ℓ/r is the retired relative rule's ratio (old line 10⁻³), kept as reported only.".into()),
        };
        tiles.push(Tile {
            col,
            span: 1,
            outcome: Outcome::from_word(&word),
            word,
            checks,
            commit: commit.clone(),
            tag: None,
            tables: vec![table],
            heat: vec![],
            m8: None,
            progress: None,
            superseded_by: None,
        });
    }
    merge_progress(&mut tiles, read_progress(&dir));
    let n_pass = tiles.iter().filter(|t| t.outcome == Outcome::Pass).count();
    let decided: Vec<Outcome> = tiles.iter().map(|t| t.outcome).collect();
    Suite {
        key: "domain".into(),
        name: name.into(),
        registration: reg.into(),
        source: src,
        verdict: format!("{n_pass}/{} PASS", tiles.len()),
        verdict_outcome: Outcome::worst(decided),
        warning: commit_warning(&dir, commit.as_deref()),
        predictions: load_predictions(&dir, Some(&r)),
        tiles,
        superseded: vec![],
        interval: false,
    }
}

fn word_of_bool(v: &Value) -> String {
    match v.as_bool() {
        Some(true) => "pass".into(),
        Some(false) => "fail".into(),
        None => "—".into(),
    }
}

fn clusters_text(v: Option<&Value>) -> String {
    let Some(a) = v.and_then(Value::as_array) else {
        return "—".into();
    };
    a.iter()
        .map(|c| {
            format!(
                "({})",
                c.as_array()
                    .map(|x| x
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(","))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn load_gluing(paths: &Paths) -> Suite {
    let dir = paths.gluing();
    let src = rel(&paths.root, &dir.join("report.json"));
    let reg = "flux-method amendment §5.5, corrected in §5.8";
    let name = "Gluing";
    let Some(r) = read_json(&dir.join("report.json")) else {
        let mut s = missing_suite(
            "gluing",
            name,
            reg,
            src,
            "report.json is missing",
            load_predictions(&dir, None),
            false,
        );
        merge_progress(&mut s.tiles, read_progress(&dir));
        return s;
    };
    let mut tiles = vec![];
    for (i, p) in r
        .get("points")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let (Some(g2), mq) = (
            p.get("g2").and_then(Value::as_f64),
            p.get("Mq").and_then(Value::as_i64),
        ) else {
            continue;
        };
        let Some(col) = column(g2, mq) else { continue };
        let word = p
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("OPEN")
            .to_string();
        let sm = |k: &str| p.pointer(&format!("/seams/{k}"));
        let holds = |k: &str| {
            Outcome::from_holds(sm(k).and_then(|s| s.get("holds")).unwrap_or(&Value::Null))
        };
        let s1 = sm("S1");
        let s1_text = ["bare", "w_over_Z", "dressed"]
            .iter()
            .map(|k| {
                format!(
                    "{k} {}",
                    clusters_text(s1.and_then(|s| s.pointer(&format!("/clusters/{k}"))))
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let s2 = Check::new("S2 quadrature", None)
            .number(&r, &format!("/points/{i}/seams/S2/max_w_tilde_change"))
            .tol(1e-5)
            .outcome(holds("S2"));
        let s3_note = sm("S3")
            .and_then(|s| s.get("note"))
            .and_then(Value::as_str)
            .map(String::from);
        let s3 = Check::new("S3 truncation", None)
            .tol(3e-3)
            .outcome(holds("S3"));
        let s3 = match &s3_note {
            Some(n) => s3.text(n.clone()),
            None => s3,
        };
        let s5 = sm("S5");
        let rho = |k: &str| s5.and_then(|s| s.get(k)).and_then(Value::as_f64);
        let tuple_text = |k: &str| {
            s5.and_then(|s| s.get(k))
                .and_then(Value::as_object)
                .map(|o| {
                    o.iter()
                        .map(|(n, v)| {
                            format!(
                                "{} {}",
                                short_state(n),
                                v.as_array()
                                    .map(|a| a.iter().map(fmt_val).collect::<Vec<_>>().join("/"))
                                    .unwrap_or_else(|| fmt_val(v))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                })
        };
        let s5_text = format!(
            "ρ {} → {}",
            rho("registered_rho").map(sci).unwrap_or("—".into()),
            rho("alternate_rho")
                .map(sci)
                .unwrap_or("not testable".into())
        );
        let mut s5c = Check::new("S5 cutoff radius", None)
            .outcome(holds("S5"))
            .text(s5_text.clone());
        s5c.value = rho("registered_rho");
        s5c.pointer = Some(format!("/points/{i}/seams/S5/registered_rho"));
        s5c.outcome = holds("S5");
        let checks = vec![
            Check::new("S1 identification", None)
                .outcome(holds("S1"))
                .text(s1_text.clone()),
            s2.clone(),
            s3.clone(),
            s5c.clone(),
        ];
        let table = Table {
            title: "the four seams".into(),
            columns: ["seam", "varied", "reading", "tolerance", "holds"].map(String::from).to_vec(),
            reported_columns: vec![],
            rows: vec![
                vec![cell("S1 identification"), cell("bare w · w/Z · dressed overlap"), cell(s1_text), cell("same clusters"), cell_o(oword(checks[0].outcome), checks[0].outcome)],
                vec![cell("S2 quadrature"), cell("order pair vs next pair"), cell(format!("max |Δw̃| {}", s2.value.map(sci).unwrap_or("—".into()))), cell("≤ 10⁻⁵"), cell_o(oword(s2.outcome), s2.outcome)],
                vec![cell("S3 truncation"), cell("Mq 6 → 8 → 10"), cell(s3_note.unwrap_or_else(|| "last step".into())), cell("≤ 0.003"), cell_o(oword(s3.outcome), s3.outcome)],
                vec![cell("S5 cutoff radius"), cell(s5_text), cell(format!("{}  ⟶  {}", tuple_text("registered").unwrap_or_default(), tuple_text("alternate").unwrap_or_default())), cell("identical"), cell_o(oword(s5c.outcome), s5c.outcome)],
            ],
            note: Some("S5 compares each state's (cut gate, state passed) and the point verdict at the two radii (§5.8).".into()),
        };
        tiles.push(Tile {
            col,
            span: 1,
            outcome: Outcome::from_word(&word),
            word,
            checks,
            commit: r.get("commit").and_then(Value::as_str).map(String::from),
            tag: None,
            tables: vec![table],
            heat: vec![],
            m8: None,
            progress: None,
            superseded_by: None,
        });
    }
    merge_progress(&mut tiles, read_progress(&dir));
    let inv = r.get("controls_invariant").and_then(Value::as_bool);
    Suite {
        key: "gluing".into(),
        name: name.into(),
        registration: reg.into(),
        source: src,
        verdict: match inv {
            Some(true) => "controls_invariant".into(),
            Some(false) => "controls not invariant".into(),
            None => "in progress".into(),
        },
        verdict_outcome: Outcome::from_holds(&Value::from(inv)),
        warning: None,
        predictions: load_predictions(&dir, Some(&r)),
        tiles,
        superseded: vec![],
        interval: false,
    }
}

fn short_state(n: &str) -> &str {
    match n {
        "helike_plus" => "H+",
        "helike_minus" => "H−",
        "competitor" => "C",
        x => x,
    }
}

pub fn oword(o: Outcome) -> String {
    match o {
        Outcome::Pass => "holds",
        Outcome::Fail => "moved",
        Outcome::Open => "open",
        Outcome::Reported => "reported",
        Outcome::NotApplicable => "n/a",
        Outcome::Queued => "queued",
        Outcome::Running => "running",
    }
    .into()
}

pub fn load_heat(paths: &Paths) -> Suite {
    let dir = paths.heat();
    let src = rel(&paths.root, &dir.join("report.json"));
    let reg = "heat-signal registration, amendments 1–2";
    let name = "Heat signal";
    let Some(r) = read_json(&dir.join("report.json")) else {
        let mut s = missing_suite(
            "heat",
            name,
            reg,
            src,
            "report.json is missing",
            load_predictions(&dir, None),
            false,
        );
        merge_progress(&mut s.tiles, read_progress(&dir));
        return s;
    };
    let mut tiles = vec![];
    for g in ["0.002", "0.004"] {
        let Some(p) = r.pointer(&format!("/points/{g}")) else {
            continue;
        };
        let col = column(g.parse().unwrap(), None).unwrap();
        let mut checks = vec![];
        let mut heat = vec![];
        let mut rows = vec![];
        for sec in ["helike", "competitor"] {
            let t1 = p.pointer(&format!("/T1/{sec}"));
            let t1w = t1
                .and_then(|t| t.get("verdict"))
                .and_then(Value::as_str)
                .unwrap_or("OPEN");
            let mut c = Check::new("T1 corridor", Some(sec))
                .outcome(Outcome::from_word(t1w))
                .text(t1w);
            let lvl = t1
                .and_then(|t| t.get("corridor_level"))
                .and_then(Value::as_i64);
            c.value = lvl.map(|l| l as f64);
            c.pointer = Some(format!("/points/{g}/T1/{sec}/corridor_level"));
            checks.push(c);
            let t1b = p
                .pointer(&format!("/T1b/{sec}"))
                .and_then(Value::as_str)
                .unwrap_or("OPEN");
            checks.push(
                Check::new("T1b vs identification", Some(sec))
                    .outcome(Outcome::from_word(t1b))
                    .text(t1b),
            );
            let base = format!("/points/{g}/T4/seams/{sec}");
            let seam_b = |k: &str| {
                Outcome::from_holds(r.pointer(&format!("{base}/{k}")).unwrap_or(&Value::Null))
            };
            let trunc = Check::new("T4 truncation", Some(sec))
                .number(&r, &format!("{base}/plateau_change_Mq"))
                .tol(3e-3)
                .outcome(seam_b("truncation_Mq"));
            let quad = Check::new("T4 quadrature", Some(sec))
                .number(&r, &format!("{base}/max_weight_change_quadrature"))
                .tol(1e-5)
                .outcome(seam_b("quadrature"));
            let lev =
                Check::new("T4 levels k8 vs k12", Some(sec)).outcome(seam_b("levels_k8_vs_k12"));
            let dress =
                Check::new("T4 source dressing", Some(sec)).outcome(seam_b("source_dressing"));
            rows.push(vec![
                cell(sec),
                cell_o(t1w, Outcome::from_word(t1w)),
                cell_o(t1b, Outcome::from_word(t1b)),
                cell_o(trunc.value.map(sci).unwrap_or("—".into()), trunc.outcome),
                cell_o(quad.value.map(sci).unwrap_or("—".into()), quad.outcome),
                cell_o(oword(lev.outcome), lev.outcome),
                cell_o(oword(dress.outcome), dress.outcome),
            ]);
            checks.extend([trunc, quad, lev, dress]);
            let mut sources = vec![];
            if let (Some(pl), Some(ms)) = (
                t1.and_then(|t| t.get("plateaus"))
                    .and_then(Value::as_object),
                t1.and_then(|t| t.get("missing")).and_then(Value::as_object),
            ) {
                for (k, v) in pl {
                    if let (Some(e), Some(m)) = (v.as_f64(), ms.get(k).and_then(Value::as_f64)) {
                        sources.push((k.clone(), e, m));
                    }
                }
            }
            heat.push(HeatSector {
                sector: sec.into(),
                verdict: Outcome::from_word(t1w),
                corridor_level: lvl,
                sources,
            });
        }
        let t4w = p
            .pointer("/T4/verdict")
            .and_then(Value::as_str)
            .unwrap_or("OPEN")
            .to_string();
        checks.push(
            Check::new("T4 verdict", None)
                .outcome(Outcome::from_word(&t4w))
                .text(t4w.clone()),
        );
        let outcome = Outcome::worst(checks.iter().map(|c| c.outcome));
        tiles.push(Tile {
            col,
            span: 2,
            outcome,
            word: if outcome == Outcome::Pass {
                "CORRIDOR · AGREES · SIGNAL-INVARIANT".into()
            } else {
                t4w
            },
            checks,
            commit: None,
            tag: Some("Mq 8 and 10".into()),
            tables: vec![Table {
                title: "T1, T1b and the T4 seams".into(),
                columns: [
                    "sector",
                    "T1",
                    "T1b",
                    "plateau Δ Mq 8→10 (≤ 0.003)",
                    "quadrature (≤ 10⁻⁵)",
                    "levels k8/k12",
                    "source dressing",
                ]
                .map(String::from)
                .to_vec(),
                reported_columns: vec![],
                rows,
                note: None,
            }],
            heat,
            m8: None,
            progress: None,
            superseded_by: None,
        });
    }
    // across the pair: T2 tone (decided) and T3 octave trend (reported only)
    if let Some(t2) = r.get("T2_controls") {
        let mut checks = vec![];
        let mut rows = vec![];
        for sec in ["helike", "competitor"] {
            let w = t2
                .pointer(&format!("/{sec}/verdict"))
                .and_then(Value::as_str)
                .unwrap_or("OPEN");
            let o = Outcome::from_word(w);
            let rc = Check::new("T2 max |Δr|", Some(sec))
                .number(&r, &format!("/T2_controls/{sec}/max_ratio_change"))
                .tol(0.02)
                .outcome(o);
            let cc = Check::new("T2 sup |Δc|", Some(sec))
                .number(&r, &format!("/T2_controls/{sec}/max_curve_change"))
                .tol(0.02)
                .outcome(o);
            let trend = r
                .pointer(&format!("/T3_controls/{sec}/trend"))
                .and_then(Value::as_str)
                .unwrap_or("—")
                .to_string();
            let t3 = Check::new("T3 octave trend", Some(sec))
                .reported()
                .text(trend.clone());
            rows.push(vec![
                cell(sec),
                cell_o(w, o),
                cell(rc.value.map(sci).unwrap_or("—".into())),
                cell(cc.value.map(sci).unwrap_or("—".into())),
                cell(trend),
            ]);
            checks.extend([rc, cc, t3]);
        }
        let outcome = Outcome::worst(checks.iter().map(|c| c.outcome));
        tiles.push(Tile {
            col: 4,
            span: 1,
            outcome,
            word: if outcome == Outcome::Pass {
                "TONE-STABLE".into()
            } else {
                oword(outcome)
            },
            checks,
            commit: None,
            tag: None,
            tables: vec![Table {
                title: "T2 tone across the pair; T3 octave trend".into(),
                columns: [
                    "sector",
                    "T2",
                    "max |Δr| (≤ 0.02)",
                    "sup |Δc| (≤ 0.02)",
                    "T3 trend",
                ]
                .map(String::from)
                .to_vec(),
                reported_columns: vec![4],
                rows,
                note: Some("T3 is reported only.".into()),
            }],
            heat: vec![],
            m8: None,
            progress: None,
            superseded_by: None,
        });
    }
    merge_progress(&mut tiles, read_progress(&dir));
    let pass = r.get("controls_pass").and_then(Value::as_bool);
    Suite {
        key: "heat".into(),
        name: name.into(),
        registration: reg.into(),
        source: src,
        verdict: match pass {
            Some(true) => "controls_pass".into(),
            Some(false) => "controls do not pass".into(),
            None => "in progress".into(),
        },
        verdict_outcome: Outcome::from_holds(&Value::from(pass)),
        warning: None,
        predictions: load_predictions(&dir, Some(&r)),
        tiles,
        superseded: vec![],
        interval: false,
    }
}

pub fn load_m8(paths: &Paths) -> Suite {
    let dir = paths.m8();
    let reg = "M8 registration, amendments 1–5";
    let dirs = m8_dirs(&dir);
    // the current report is the latest controls directory that holds a report.json (controls-a5 after controls-a4)
    let current = dirs.iter().rev().map(|d| d.join("report.json")).find(|p| p.exists());
    let Some(path) = current else {
        let mut s = missing_suite("m8", "M8", reg, "research/m8-20261126/controls*/report.json".into(), "no controls report on this checkout", load_predictions(&dir, None), false);
        let mut prog = vec![];
        for d in &dirs {
            prog.extend(read_progress(d));
        }
        merge_progress(&mut s.tiles, prog);
        return s;
    };
    let src = rel(&paths.root, &path);
    let Some(r) = read_json(&path) else {
        return missing_suite("m8", "M8", reg, src, "report.json does not parse", load_predictions(&dir, None), false);
    };
    let mut tiles = vec![];
    for (i, p) in r.get("points").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let (Some(g2), mq) = (p.get("g2").and_then(Value::as_f64), p.get("Mq").and_then(Value::as_i64)) else { continue };
        let Some(col) = column(g2, mq) else { continue };
        let status = p.get("status").and_then(Value::as_str).unwrap_or("OPEN").to_string();
        let orders: Vec<i64> = p.get("seam_orders").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
        let mut checks = vec![];
        let mut rows = vec![];
        let mut channels = vec![];
        for ch in ["helike", "competitor"] {
            let base = format!("/points/{i}/{ch}");
            let c = p.get(ch);
            let gate = |k: &str| Outcome::from_holds(c.and_then(|c| c.pointer(&format!("/gates/{k}"))).unwrap_or(&Value::Null));
            let label = c.and_then(|c| c.get("label")).and_then(Value::as_str).unwrap_or("OPEN").to_string();
            let own = Check::new("own share", Some(ch)).number(&r, &format!("{base}/own_share")).outcome(Outcome::from_word(&label)).text(label.clone());
            let q0a = Check::new("Q0a identity", Some(ch)).number(&r, &format!("{base}/identity_rel_error")).tol(1e-4).outcome(gate("Q0a_identity"));
            let q0b = Check::new("Q0b order seam", Some(ch)).number(&r, &format!("{base}/share_seam")).tol(1e-3).outcome(gate("Q0b_quadrature"));
            let cut = Check::new("A5 cut seam", Some(ch)).number(&r, &format!("{base}/cut_seam")).tol(1e-3).outcome(gate("A5_cut_seam"));
            let exc = Check::new("excluded 1e-12", Some(ch)).number(&r, &format!("{base}/excluded_fraction/1e-12")).tol(1e-3).outcome(gate("A5_cut_seam"));
            let q2 = Check::new("Q2 cross-check", Some(ch)).number(&r, &format!("{base}/q2_difference")).outcome(gate("Q2_cross_check"));
            let pos = Check::new("positivity", Some(ch)).outcome(gate("positivity"));
            let res = Check::new("residual share", Some(ch)).number(&r, &format!("{base}/residual_share")).tol(1e-2).reported();
            rows.push(vec![
                cell(ch),
                cell_o(own.value.map(sci).unwrap_or("—".into()), own.outcome),
                cell_o(q0a.value.map(sci).unwrap_or("—".into()), q0a.outcome),
                cell_o(q0b.value.map(sci).unwrap_or("—".into()), q0b.outcome),
                cell_o(cut.value.map(sci).unwrap_or("—".into()), cut.outcome),
                cell_o(exc.value.map(sci).unwrap_or("—".into()), exc.outcome),
                cell_o(q2.value.map(sci).unwrap_or("—".into()), q2.outcome),
                cell_o(oword(pos.outcome), pos.outcome),
                cell(res.value.map(sci).unwrap_or("—".into())),
            ]);
            checks.extend([own, q0a, q0b, cut, exc, q2, pos, res]);
            // geometry for the detail panels, both orders of the seam pair
            let shares: Vec<(i64, [f64; 4])> = orders
                .iter()
                .filter_map(|o| {
                    let sh = p.pointer(&format!("/orders/{o}/{ch}/shares"))?;
                    let g = |k: &str| sh.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                    Some((*o, [g("H"), g("C"), g("S"), g("R")]))
                })
                .collect();
            let fine = orders.last().copied().unwrap_or(0);
            let raw: Vec<[[f64; 4]; 4]> = p
                .pointer(&format!("/orders/{fine}/{ch}/raw"))
                .and_then(Value::as_array)
                .map(|levels| {
                    levels
                        .iter()
                        .map(|m| {
                            let mut out = [[0.0; 4]; 4];
                            for (a, row) in m.as_array().into_iter().flatten().enumerate().take(4) {
                                for (b, v) in row.as_array().into_iter().flatten().enumerate().take(4) {
                                    out[a][b] = v.as_f64().unwrap_or(0.0);
                                }
                            }
                            out
                        })
                        .collect()
                })
                .unwrap_or_default();
            let block = c.and_then(|c| c.get("block")).and_then(Value::as_str).unwrap_or("").to_string();
            channels.push(M8Channel {
                name: ch.into(),
                shape_price: p.pointer(&format!("/shape_price/{block}")).and_then(Value::as_f64),
                discount: p.pointer(&format!("/dressing_discount/{block}")).and_then(Value::as_f64),
                gap: c.and_then(|c| c.get("gap")).and_then(Value::as_f64),
                block,
                label,
                shares,
                raw,
            });
        }
        let ordering_shapes = p.pointer("/ordering/shapes").and_then(Value::as_i64);
        let ordering_levels = p.pointer("/ordering/levels").and_then(Value::as_i64);
        let open: Vec<String> = p.get("open_reasons").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        let table = Table {
            title: "the M8 gates, per channel".into(),
            // two-line headers so the table fits its panel
            columns: ["channel", "own\nshare", "Q0a\n≤ 10⁻⁴", "Q0b seam\n≤ 10⁻³", "cut seam\n≤ 10⁻³", "excluded\n≤ 10⁻³", "Q2 vs\ndressed", "posi-\ntivity", "residual\nshare"]
                .map(String::from)
                .to_vec(),
            reported_columns: vec![8],
            rows,
            note: Some(if open.is_empty() {
                "Q2 is a cross-check (not decided on); the residual share is reported, flagged only above 10⁻².".to_string()
            } else {
                format!("open: {}", open.join("; "))
            }),
        };
        let word = status.clone();
        tiles.push(Tile {
            col,
            span: 1,
            outcome: Outcome::from_word(&status),
            word,
            checks,
            commit: None,
            tag: (orders.len() == 2).then(|| format!("{}/{} · A4", orders[0], orders[1])),
            tables: vec![table],
            heat: vec![],
            m8: Some(M8Detail { channels, ordering_shapes, ordering_levels }),
            progress: None,
            superseded_by: None,
        });
    }
    let mut prog = vec![];
    for d in &dirs {
        prog.extend(read_progress(d));
    }
    merge_progress(&mut tiles, prog);
    let n_res = tiles.iter().filter(|t| t.outcome == Outcome::Pass).count();
    Suite {
        key: "m8".into(),
        name: "M8".into(),
        registration: reg.into(),
        source: src,
        verdict: format!("{n_res}/{} RESOLVED", tiles.len()),
        verdict_outcome: Outcome::worst(tiles.iter().map(|t| t.outcome)),
        warning: None,
        predictions: load_predictions(&dir, Some(&r)),
        tiles,
        superseded: vec![],
        interval: false,
    }
}

pub fn load_r2(paths: &Paths) -> Suite {
    let dir = paths.r2();
    let reg = "S_C R2 registration";
    let files = r2_files(&dir);
    let src = files
        .first()
        .map(|p| rel(&paths.root, p))
        .unwrap_or_else(|| "research/sc-r2-20261126/r2_g*.txt".into());
    let mut s = missing_suite(
        "r2",
        "S_C · R2",
        reg,
        src,
        "no R2 run log on this checkout",
        load_predictions(&dir, None),
        true,
    );
    for f in files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "txt"))
    {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let g2: Option<f64> = name
            .trim_start_matches("r2_g")
            .trim_end_matches(".txt")
            .parse()
            .ok();
        let Some(col) = g2.and_then(|g| column(g, None)) else {
            continue;
        };
        let text = std::fs::read_to_string(f).unwrap_or_default();
        // the stage log: the unindented lines, in order
        let stages: Vec<Vec<Cell>> = text
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with(char::is_whitespace))
            .map(|l| vec![cell(l.trim())])
            .collect();
        let last = stages.last().map(|r| r[0].text.clone()).unwrap_or_default();
        s.tiles.push(Tile {
            col,
            span: 2,
            outcome: Outcome::Running,
            word: "stage log".into(),
            checks: vec![Check::new("stage", None).reported().text(last)],
            commit: None,
            tag: Some(name),
            tables: vec![Table {
                title: "stage log".into(),
                columns: vec!["stage".into()],
                reported_columns: vec![],
                rows: stages,
                note: Some(
                    "The interval ℓ₅ against ρ_C is drawn when the result file lands.".into(),
                ),
            }],
            heat: vec![],
            m8: None,
            progress: None,
            superseded_by: None,
        });
    }
    if !s.tiles.is_empty() {
        s.warning = None;
        s.verdict = format!("{} running", s.tiles.len());
    }
    s
}

pub fn now_unix() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// ------------------------------------------------------------------------------------------------ plain English

/// What each test suite does, in a few plain sentences.
pub fn about_suite(key: &str) -> &'static str {
    match key {
        "domain" => "The solver works inside a finite box. This test measures how much of each state reaches the edge of that box, and how far cutting the state off there could move its energy. It passes when that shift is less than a tenth of the accuracy the grid reports energies to.",
        "gluing" => "Every result is stitched together from choices: how Helike is picked out, how finely the integrals are sampled, how large the basis is, and where the box is cut. This test changes each of those choices on purpose and checks that the answer stays put. When it does, the answer belongs to the physics, not to the choices.",
        "heat" => "Each source is a different way of plucking the system. The heat signal follows how fast each one rings down. When every source rings down to the same lowest note, that note is the corridor. The tone compares the overtones above that note at two couplings.",
        "m8" => "This test asks where each state's energy lives: in bending along Helike's own directions, the competitor's, or the rest. A state is aligned when most of its energy sits in its own block. The other checks confirm that the bookkeeping adds back up to the solver's energy, at two samplings and two cutoffs.",
        "r2" => "This is the rigorous lower-bound calculation for the separation between Helike and her competitors. It uses interval arithmetic, so every rounding error is carried along and accounted for.",
        _ => "",
    }
}

/// What one check does, in one plain sentence.
pub fn about_check(name: &str) -> &'static str {
    match name {
        "e_cut" => "How far cutting the state at the edge of the box could move its energy. It must stay under 3·10⁻⁴.",
        "tail² budget" => "How much of the state lies outside the region the bounds cover. It must stay under 10⁻¹².",
        "norm width" => "How tightly the size of the state is pinned down. The uncertainty must stay under 10⁻¹⁰.",
        "ℓ/r" => "The leakage divided by the state's residual: the old rule, kept on screen for reference.",
        "r upper" => "The guaranteed upper bound on how far the state is from an exact solution.",
        "S1 identification" => "Helike is picked out three different ways; all three must land on the same levels.",
        "S2 quadrature" => "The integrals are sampled more finely; the weights must move by no more than 10⁻⁵.",
        "S3 truncation" => "The basis grows from Mq 8 to Mq 10; the energies must move by no more than 0.003.",
        "S5 cutoff radius" => "The box is cut at a second radius; the domain gate must give the same verdict.",
        "T1 corridor" => "Every source in the sector reaches the same lowest note.",
        "T1b vs identification" => "The corridor's note is the same level the grid identifies by name.",
        "T4 truncation" => "The note moves by no more than 0.003 when the basis grows from Mq 8 to Mq 10.",
        "T4 quadrature" => "The weights move by no more than 10⁻⁵ when the integrals are sampled more finely.",
        "T4 levels k8 vs k12" => "Reading the signal from 8 levels or from 12 gives the same note.",
        "T4 source dressing" => "The bare source and the registered source find the same note.",
        "T4 verdict" => "All the seams of the heat signal hold together.",
        "T2 max |Δr|" => "The overtone ratios at the two couplings differ by no more than 0.02.",
        "T2 sup |Δc|" => "The whole ring-down curves at the two couplings differ by no more than 0.02.",
        "T3 octave trend" => "Whether the lowest note rises or falls as the coupling grows.",
        "own share" => "The fraction of the state's energy that sits in its own block.",
        "Q0a identity" => "The energy rebuilt from the state's shape equals the solver's energy, to one part in 10⁴.",
        "Q0b order seam" => "The integrals are sampled more finely; the shares must move by no more than 10⁻³.",
        "A5 cut seam" => "The shares are read with two cutoffs where the ground state vanishes; they must agree to 10⁻³.",
        "excluded 1e-12" => "How much of the state lies in the region left out by the cutoff. It must stay under 10⁻³.",
        "Q2 cross-check" => "A second, independent route to the overlap the solver reports; the two must match.",
        "positivity" => "The ground state is positive everywhere it is read.",
        "residual share" => "The part of the energy that comes from the ground state being approximate.",
        "stage" => "The most recent step the run has reached.",
        _ => "",
    }
}

// ------------------------------------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> Paths {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        Paths {
            root: find_root(here).expect("repo root above the crate"),
        }
    }

    /// Acceptance 1: every value the scene would draw equals its report value, bit for bit.
    #[test]
    fn numbers_match_receipts() {
        let p = paths();
        let m8_report = m8_dirs(&p.m8()).iter().rev().map(|d| d.join("report.json")).find(|x| x.exists()).expect("M8 report present on this branch");
        for (s, report) in [
            (load_domain(&p), p.domain().join("report.json")),
            (load_gluing(&p), p.gluing().join("report.json")),
            (load_heat(&p), p.heat().join("report.json")),
            (load_m8(&p), m8_report),
        ] {
            let r = read_json(&report).expect("report present on this branch");
            let mut n = 0;
            for t in &s.tiles {
                for c in &t.checks {
                    if let (Some(v), Some(ptr)) = (c.value, &c.pointer) {
                        let src = r
                            .pointer(ptr)
                            .and_then(Value::as_f64)
                            .unwrap_or_else(|| panic!("{} {ptr} not in report", s.key));
                        assert_eq!(v.to_bits(), src.to_bits(), "{} {} {ptr}", s.key, c.name);
                        n += 1;
                    }
                }
            }
            assert!(n >= 8, "{}: only {n} values checked", s.key);
        }
    }

    #[test]
    fn domain_tiles_follow_the_report() {
        let s = load_domain(&paths());
        assert_eq!(s.tiles.len(), 4);
        assert!(s.tiles.iter().all(|t| t.outcome == Outcome::Pass));
        assert_eq!(s.verdict, "4/4 PASS");
        // the g² 0.004 Mq 10 tile: the old ratio above 1e-3, e_cut far inside 3e-4
        let t = s.tiles.iter().find(|t| t.col == 3).unwrap();
        let ratio = t
            .checks
            .iter()
            .find(|c| c.name == "ℓ/r" && c.group.as_deref() == Some("helike_plus"))
            .unwrap();
        assert!(ratio.value.unwrap() > 1e-3);
        assert_eq!(ratio.outcome, Outcome::Reported);
    }

    /// Acceptance 2: OPEN is never green; FAIL is FAIL.
    #[test]
    fn open_is_never_pass() {
        assert_eq!(Outcome::from_word("OPEN"), Outcome::Open);
        assert_eq!(Outcome::from_word("something new"), Outcome::Open);
        assert_eq!(Outcome::from_word("FAIL"), Outcome::Fail);
        assert_eq!(
            Outcome::worst([Outcome::Pass, Outcome::Open, Outcome::Pass]),
            Outcome::Open
        );
        assert_eq!(
            Outcome::worst([Outcome::Pass, Outcome::Open, Outcome::Fail]),
            Outcome::Fail
        );
        assert_eq!(Outcome::worst([]), Outcome::Open);
    }

    /// Acceptance 3: reported-only values never colour a tile.
    #[test]
    fn reported_values_never_decide() {
        assert_eq!(
            Outcome::worst([Outcome::Pass, Outcome::Reported, Outcome::NotApplicable]),
            Outcome::Pass
        );
        let p = paths();
        for s in [load_domain(&p), load_heat(&p)] {
            for t in &s.tiles {
                for c in t.checks.iter().filter(|c| c.reported_only) {
                    assert_eq!(c.outcome, Outcome::Reported, "{} {}", s.key, c.name);
                }
            }
        }
        let heat = load_heat(&p);
        assert!(heat
            .tiles
            .iter()
            .flat_map(|t| &t.checks)
            .any(|c| c.name == "T3 octave trend" && c.reported_only));
    }

    /// Acceptance 5: a missing receipt is visible.
    #[test]
    fn missing_receipt_is_flagged() {
        let tmp = std::env::temp_dir().join(format!("helike-view-empty-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("research")).unwrap();
        let p = Paths { root: tmp.clone() };
        for s in [
            load_domain(&p),
            load_gluing(&p),
            load_heat(&p),
            load_m8(&p),
            load_r2(&p),
        ] {
            assert!(s.warning.is_some(), "{} should warn", s.key);
            assert!(s.tiles.is_empty());
        }
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn progress_makes_running_tiles() {
        let tmp = std::env::temp_dir().join(format!("helike-view-prog-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("progress.jsonl"),
            "{\"suite\":\"M8\",\"g2\":0.004,\"Mq\":10,\"stage\":\"export\",\"t\":1.0}\n{\"suite\":\"M8\",\"g2\":0.004,\"Mq\":10,\"stage\":\"order 14\",\"t\":2.0}\n{\"g2\":0.002,\"Mq\":8,\"stage\":\"done\",\"t\":3.0}\n",
        )
        .unwrap();
        let mut tiles = vec![];
        merge_progress(&mut tiles, read_progress(&tmp));
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[0].col, 0);
        assert_eq!(tiles[0].outcome, Outcome::Queued);
        assert_eq!(tiles[1].outcome, Outcome::Running);
        assert_eq!(tiles[1].progress.as_ref().unwrap().stage, "order 14");
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn predictions_compare_as_written() {
        let r: Value = serde_json::json!({"a": 5.58e-6, "v": "PASS", "w": "PASS", "x": 0.95});
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::RoundsTo {
                    pointer: "/a".into(),
                    value: 5.6e-6,
                    sig: 2
                }
            )
            .0,
            Mark::Met
        );
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::RoundsTo {
                    pointer: "/a".into(),
                    value: 5.5e-6,
                    sig: 2
                }
            )
            .0,
            Mark::Missed
        );
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::Equals {
                    pointers: vec!["/v".into(), "/w".into()],
                    value: "PASS".into()
                }
            )
            .0,
            Mark::Met
        );
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::Ge {
                    pointers: vec!["/x".into()],
                    value: 0.9
                }
            )
            .0,
            Mark::Met
        );
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::Le {
                    pointers: vec!["/x".into()],
                    value: 0.9
                }
            )
            .0,
            Mark::Missed
        );
        assert_eq!(
            evaluate(
                &r,
                &PredCheck::Le {
                    pointers: vec!["/nope".into()],
                    value: 0.9
                }
            )
            .0,
            Mark::NotYetRun
        );
    }

    /// Every suite and every check the board shows has a plain-English line.
    #[test]
    fn every_test_is_explained() {
        let (suites, _) = load_all(&paths());
        for s in &suites {
            assert!(!about_suite(&s.key).is_empty(), "suite {}", s.key);
            for t in &s.tiles {
                for c in &t.checks {
                    assert!(!about_check(&c.name).is_empty(), "check {} in {}", c.name, s.key);
                }
            }
        }
    }

    /// Acceptance 4 on M8 itself: the pre-amendment-4 Mq 10 tile and the amendment-4 g² 0.004 tile are kept, OPEN,
    /// struck through and linked to the amendment that replaced them; the live tiles are the amendment-5 report's.
    #[test]
    fn m8_superseded_tiles_are_kept() {
        let (suites, _) = load_all(&paths());
        let m8 = suites.iter().find(|s| s.key == "m8").unwrap();
        assert_eq!(m8.tiles.len(), 4);
        assert!(m8.tiles.iter().all(|t| t.outcome == Outcome::Pass && t.m8.is_some()));
        let sup: Vec<(usize, Outcome, Option<String>)> = m8.superseded.iter().map(|t| (t.col, t.outcome, t.superseded_by.clone())).collect();
        assert!(sup.contains(&(1, Outcome::Open, Some("M8 A4".into()))), "{sup:?}");
        assert!(sup.contains(&(3, Outcome::Open, Some("M8 A5".into()))), "{sup:?}");
        assert_eq!(m8.verdict, "4/4 RESOLVED");
    }

    /// Acceptance 4 (mechanism): a superseded tile from the history file is kept beside the live one.
    #[test]
    fn superseded_tiles_are_kept() {
        let (suites, history) = load_all(&paths());
        if history.iter().any(|h| !h.supersedes.is_empty()) {
            let n: usize = suites.iter().map(|s| s.superseded.len()).sum();
            assert!(n > 0);
            assert!(suites
                .iter()
                .flat_map(|s| &s.superseded)
                .all(|t| t.superseded_by.is_some()));
        }
    }
}
