//! GIGI CLI + REPL — interactive database shell.
//!
//! Usage:
//!   gigi                    Start interactive REPL
//!   gigi --dir <path>       Use specified data directory
//!   gigi -e "QUERY"         Execute a single query and exit

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut data_dir = PathBuf::from("gigi_data");
    let mut exec_query: Option<String> = None;

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
            "--help" | "-h" => {
                println!("GIGI — Geometric Intrinsic Global Index");
                println!();
                println!("Usage:");
                println!("  gigi                    Start interactive REPL");
                println!("  gigi --dir <path>       Use specified data directory");
                println!("  gigi -e \"QUERY\"         Execute a single query and exit");
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

    // Interactive REPL
    println!("GIGI v0.3 — Geometric Intrinsic Global Index");
    println!("Type .help for commands, .quit to exit.\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("gigi> ");
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match execute_line(&mut engine, line) {
            Ok(true) => break, // quit signal
            Ok(false) => {}
            Err(e) => eprintln!("Error: {e}"),
        }
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
                println!("Bundle: {}", store.schema.name);
                println!("Base fields:");
                for f in &store.schema.base_fields {
                    println!("  {} {:?} (range: {:?})", f.name, f.field_type, f.range);
                }
                println!("Fiber fields:");
                for f in &store.schema.fiber_fields {
                    println!("  {} {:?} (range: {:?})", f.name, f.field_type, f.range);
                }
                println!("Indexed: {:?}", store.schema.indexed_fields);
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
                let k = gigi::curvature::scalar_curvature(store);
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
    }

    Ok(false)
}
