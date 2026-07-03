//! GIGI CLI + REPL — interactive database shell.
//!
//! Usage:
//!   gigi                    Start interactive REPL
//!   gigi --dir <path>       Use specified data directory
//!   gigi -e "QUERY"         Execute a single query and exit

use std::path::PathBuf;

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut data_dir = PathBuf::from("gigi_data");
    let mut exec_query: Option<String> = None;
    let mut script_file: Option<PathBuf> = None;
    let mut keep_going = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" | "-d" => {
                i += 1;
                if i < args.len() {
                    data_dir = PathBuf::from(&args[i]);
                } else {
                    eprintln!("Error: --dir requires a path argument");
                    std::process::exit(1);
                }
            }
            "-e" | "--exec" => {
                i += 1;
                if i < args.len() {
                    exec_query = Some(args[i].clone());
                } else {
                    eprintln!("Error: -e requires a query argument");
                    std::process::exit(1);
                }
            }
            "-f" | "--file" => {
                i += 1;
                if i < args.len() {
                    script_file = Some(PathBuf::from(&args[i]));
                } else {
                    eprintln!("Error: -f requires a file path");
                    std::process::exit(1);
                }
            }
            "--keep-going" | "-k" => keep_going = true,
            "--help" | "-h" => {
                println!("GIGI — Geometric Intrinsic Global Index");
                println!();
                println!("Usage:");
                println!("  gigi                    Start interactive REPL");
                println!("  gigi --dir <path>       Use specified data directory");
                println!("  gigi -e \"QUERY\"         Execute a single query and exit");
                println!("  gigi -f script.gql      Run a file of ;-separated statements");
                println!("  gigi -f s.gql -k        …continuing past errors (--keep-going)");
                println!();
                println!("GQL Statements:");
                println!("  CREATE BUNDLE name (field TYPE [BASE|FIBER] [INDEX], ...)");
                println!("  INSERT INTO name (col, ...) VALUES (val, ...)");
                println!("  SELECT cols FROM name [WHERE cond] [GROUP BY field]");
                println!("  SELECT cols FROM left JOIN right ON field");
                println!("  CURVATURE name");
                println!("  SPECTRAL name");
                println!();
                println!("REPL Commands:");
                println!("  .bundles    List all bundles");
                println!("  .schema N   Show schema for bundle N");
                println!("  .quit       Exit");
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let mut engine = match Engine::open(&data_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to open database at {}: {e}", data_dir.display());
            std::process::exit(1);
        }
    };

    if let Some(query) = exec_query {
        match execute_line(&mut engine, &query) {
            Ok(true) => {}
            Ok(false) => {}
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(path) = script_file {
        std::process::exit(run_script(&mut engine, &path, keep_going));
    }

    // Interactive REPL
    println!("GIGI v0.3 — Geometric Intrinsic Global Index");
    println!("Type .help for commands, .quit to exit.\n");

    // rustyline: arrow-key editing, ↑/↓ history, Ctrl-C clears the
    // line, Ctrl-D exits. History persists next to the data so each
    // database remembers its own conversation.
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("Could not initialize line editor: {e}");
            std::process::exit(1);
        }
    };
    let history_path = data_dir.join(".gigi_history");
    let _ = rl.load_history(&history_path); // absent on first run — fine

    loop {
        match rl.readline("gigi> ") {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&line);
                match execute_line(&mut engine, &line) {
                    Ok(true) => break, // quit signal
                    Ok(false) => {}
                    Err(e) => eprintln!("Error: {e}"),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => continue, // Ctrl-C: new prompt
            Err(rustyline::error::ReadlineError::Eof) => break,            // Ctrl-D: exit
            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }
    }
    if let Err(e) = rl.save_history(&history_path) {
        eprintln!("(could not save history: {e})");
    }
}

/// Split a GQL script into statements on `;`, respecting single-quoted
/// strings and `--` line comments. Returns (starting_line_number, text)
/// pairs so errors can point at the file, not the void.
fn split_script(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut cur_start_line = 1usize;
    let mut line_no = 1usize;
    let mut in_str = false;
    let mut in_comment = false;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            line_no += 1;
            in_comment = false;
            cur.push(' ');
            continue;
        }
        if in_comment {
            continue;
        }
        if !in_str && c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            in_comment = true;
            continue;
        }
        if c == '\'' {
            in_str = !in_str;
        }
        if c == ';' && !in_str {
            cur.push(';');
            if !cur.trim().is_empty() {
                out.push((cur_start_line, cur.trim().to_string()));
            }
            cur.clear();
            cur_start_line = line_no;
            continue;
        }
        if cur.trim().is_empty() && !c.is_whitespace() {
            cur_start_line = line_no;
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push((cur_start_line, cur.trim().to_string()));
    }
    out
}

/// Run a `.gql` script: every statement in order, errors tagged with
/// file:line. Stops at the first error unless `keep_going`. Returns a
/// process exit code (number of failed statements, capped at 1 when
/// stopping early).
fn run_script(engine: &mut Engine, path: &std::path::Path, keep_going: bool) -> i32 {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let statements = split_script(&src);
    if statements.is_empty() {
        eprintln!("{}: no statements found", path.display());
        return 1;
    }
    let total = statements.len();
    let mut failed = 0usize;
    for (idx, (line_no, stmt)) in statements.iter().enumerate() {
        match execute_line(engine, stmt) {
            Ok(_) => {}
            Err(e) => {
                failed += 1;
                eprintln!(
                    "{}:{}: statement {}/{} failed:\n  {}\n  -> {e}",
                    path.display(),
                    line_no,
                    idx + 1,
                    total,
                    stmt
                );
                if !keep_going {
                    eprintln!("(stopping — pass --keep-going to continue past errors)");
                    return 1;
                }
            }
        }
    }
    if failed == 0 {
        println!("{}: {total} statement(s) OK", path.display());
        0
    } else {
        eprintln!("{}: {failed}/{total} statement(s) failed", path.display());
        1
    }
}

/// Execute a line of input. Returns Ok(true) if the REPL should quit.
fn execute_line(engine: &mut Engine, line: &str) -> Result<bool, String> {
    // REPL dot-commands
    if line.starts_with('.') {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            ".quit" | ".exit" | ".q" => return Ok(true),
            ".help" | ".h" => {
                println!("Commands:");
                println!("  .bundles        List all bundles");
                println!("  .schema <name>  Show schema for a bundle");
                println!("  .stats <name>   Show field stats");
                println!("  .quit           Exit");
                return Ok(false);
            }
            ".bundles" | ".tables" => {
                let names = engine.bundle_names();
                if names.is_empty() {
                    println!("(no bundles)");
                } else {
                    for name in names {
                        println!("  {name}");
                    }
                }
                return Ok(false);
            }
            ".schema" => {
                if parts.len() < 2 {
                    return Err("Usage: .schema <bundle_name>".into());
                }
                let name = parts[1];
                let store = engine
                    .bundle(name)
                    .ok_or_else(|| format!("No bundle: {name}"))?;
                println!("Bundle: {}", store.schema().name);
                println!("Base fields:");
                for f in &store.schema().base_fields {
                    println!("  {} {:?} (range: {:?})", f.name, f.field_type, f.range);
                }
                println!("Fiber fields:");
                for f in &store.schema().fiber_fields {
                    println!("  {} {:?} (range: {:?})", f.name, f.field_type, f.range);
                }
                println!("Indexed: {:?}", store.schema().indexed_fields);
                println!("Records: {}", store.len());
                return Ok(false);
            }
            ".stats" => {
                if parts.len() < 2 {
                    return Err("Usage: .stats <bundle_name>".into());
                }
                let name = parts[1];
                let store = engine
                    .bundle(name)
                    .ok_or_else(|| format!("No bundle: {name}"))?;
                let k = store.scalar_curvature();
                let conf = gigi::curvature::confidence(k);
                println!("Records: {}", store.len());
                println!("Curvature K: {k:.6}");
                println!("Confidence: {conf:.6}");
                return Ok(false);
            }
            _ => return Err(format!("Unknown command: {}", parts[0])),
        }
    }

    // Parse and execute GQL
    let stmt = parser::parse(line)?;
    let result = parser::execute(engine, &stmt)?;

    match result {
        ExecResult::Ok => println!("OK"),
        ExecResult::Notice(msg) => println!("NOTICE: {msg}"),
        ExecResult::Rows(rows) => {
            if rows.is_empty() {
                println!("(0 rows)");
            } else {
                // Collect all column names
                let mut cols: Vec<String> = rows[0].keys().cloned().collect();
                cols.sort();
                // Print header
                println!("{}", cols.join(" | "));
                println!(
                    "{}",
                    cols.iter()
                        .map(|c| "-".repeat(c.len().max(6)))
                        .collect::<Vec<_>>()
                        .join("-+-")
                );
                // Print rows
                for row in &rows {
                    let vals: Vec<String> = cols
                        .iter()
                        .map(|c| row.get(c).map_or("NULL".into(), |v| format!("{v}")))
                        .collect();
                    println!("{}", vals.join(" | "));
                }
                println!("({} rows)", rows.len());
            }
        }
        ExecResult::Scalar(v) => println!("{v:.6}"),
        ExecResult::Bool(b) => println!("{b}"),
        ExecResult::Count(n) => println!("{n} affected"),
        ExecResult::Stats(stats) => {
            println!("curvature: {:.6}", stats.curvature);
            println!("confidence: {:.2}%", stats.confidence * 100.0);
            println!("records: {}", stats.record_count);
            println!("storage: {}", stats.storage_mode);
            println!(
                "base fields: {}, fiber fields: {}",
                stats.base_fields, stats.fiber_fields
            );
        }
        ExecResult::Bundles(infos) => {
            for info in &infos {
                println!(
                    "{} ({} records, {} fields)",
                    info.name, info.records, info.fields
                );
            }
            println!("({} bundles)", infos.len());
        }
        ExecResult::Invariants(results) => {
            for (label, value) in &results {
                println!("{label}: {value:.6}");
            }
        }
    }

    Ok(false)
}
