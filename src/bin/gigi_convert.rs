//! GIGI Convert CLI — Convert JSON/CSV to DHOOM format
//!
//! Usage:
//!   gigi-convert input.json -o output.dhoom
//!   gigi-convert input.csv --format csv -o output.dhoom
//!   gigi-convert input.json --profile
//!   gigi-convert output.dhoom --to json -o roundtrip.json
//!   cat data.jsonl | gigi-convert --stream -o output.dhoom

use clap::Parser;
use std::io::{self, Read, Write, BufRead, BufWriter};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gigi-convert")]
#[command(about = "Convert JSON/CSV to DHOOM format — geometric data profiling")]
#[command(version = "0.1.0")]
#[command(author = "Davis Geometric")]
struct Cli {
    /// Input file (omit for stdin)
    input: Option<PathBuf>,

    /// Output file (omit for stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Input format: json, csv, dhoom (auto-detected from extension)
    #[arg(long)]
    format: Option<String>,

    /// Output format when decoding DHOOM: json
    #[arg(long)]
    to: Option<String>,

    /// Profile only — analyze without producing output
    #[arg(long)]
    profile: bool,

    /// Collection name (default: derived from filename)
    #[arg(long)]
    name: Option<String>,

    /// Streaming mode for line-delimited JSON
    #[arg(long)]
    stream: bool,

    /// Override default field: --default "field=value"
    #[arg(long)]
    default: Vec<String>,

    /// Force arithmetic on field: --arithmetic "field@start+step"
    #[arg(long)]
    arithmetic: Vec<String>,
}

fn detect_format(path: &PathBuf) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") | Some("jsonl") => "json".to_string(),
        Some("csv") => "csv".to_string(),
        Some("dhoom") => "dhoom".to_string(),
        _ => "json".to_string(),
    }
}

fn collection_name(cli: &Cli) -> String {
    if let Some(name) = &cli.name {
        return name.clone();
    }
    if let Some(path) = &cli.input {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            return stem.to_string();
        }
    }
    "data".to_string()
}

fn main() {
    let cli = Cli::parse();

    // Read input
    let input_text = if let Some(path) = &cli.input {
        fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", path.display(), e);
            std::process::exit(1);
        })
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            std::process::exit(1);
        });
        buf
    };

    let format = cli.format.clone()
        .unwrap_or_else(|| {
            cli.input.as_ref()
                .map(|p| detect_format(p))
                .unwrap_or_else(|| "json".to_string())
        });

    let collection = collection_name(&cli);

    // Handle DHOOM → JSON decoding
    if format == "dhoom" || cli.to.is_some() {
        let records = gigi::convert::decode_to_json(&input_text).unwrap_or_else(|e| {
            eprintln!("Error decoding DHOOM: {}", e);
            std::process::exit(1);
        });
        let json_out = serde_json::to_string_pretty(&records).unwrap();
        write_output(&cli.output, &json_out);
        eprintln!("Decoded {} records from DHOOM", records.len());
        return;
    }

    // Handle streaming mode
    if cli.stream {
        run_streaming(&cli, &collection);
        return;
    }

    // Parse input
    let json_array: Vec<serde_json::Value> = match format.as_str() {
        "csv" => {
            let result = gigi::convert::encode_csv(&input_text, &collection);
            match result {
                Ok(enc) => {
                    if cli.profile {
                        // For CSV profile, parse back to JSON first
                        let decoded = gigi::convert::decode_to_json(&enc.dhoom).unwrap();
                        let p = gigi::convert::profile(&decoded, &collection);
                        print!("{}", gigi::convert::format_profile(&p));
                        return;
                    }
                    write_output(&cli.output, &enc.dhoom);
                    eprintln!("Encoded {} bytes → {} bytes ({:.1}% compression)",
                        enc.json_bytes, enc.dhoom_bytes, enc.compression_pct);
                    return;
                }
                Err(e) => {
                    eprintln!("Error parsing CSV: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            // JSON input
            serde_json::from_str::<Vec<serde_json::Value>>(&input_text)
                .unwrap_or_else(|e| {
                    eprintln!("Error parsing JSON: {}", e);
                    std::process::exit(1);
                })
        }
    };

    // Profile mode
    if cli.profile {
        let p = gigi::convert::profile(&json_array, &collection);
        print!("{}", gigi::convert::format_profile(&p));
        return;
    }

    // Encode
    let result = gigi::convert::encode_json(&json_array, &collection);
    write_output(&cli.output, &result.dhoom);
    eprintln!("Encoded {} records | {} → {} bytes | {:.1}% compression",
        json_array.len(), result.json_bytes, result.dhoom_bytes, result.compression_pct);
}

fn run_streaming(cli: &Cli, collection: &str) {
    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut records: Vec<serde_json::Value> = Vec::new();
    let mut line_count = 0usize;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(val) => {
                records.push(val);
                line_count += 1;
            }
            Err(e) => {
                eprintln!("Warning: skipping line {}: {}", line_count + 1, e);
            }
        }
    }

    if records.is_empty() {
        eprintln!("No records read from stream");
        return;
    }

    let result = gigi::convert::encode_json(&records, collection);
    write_output(&cli.output, &result.dhoom);
    eprintln!("Stream: {} records | {:.1}% compression",
        line_count, result.compression_pct);
}

fn write_output(output: &Option<PathBuf>, content: &str) {
    if let Some(path) = output {
        let file = fs::File::create(path).unwrap_or_else(|e| {
            eprintln!("Error creating {}: {}", path.display(), e);
            std::process::exit(1);
        });
        let mut writer = BufWriter::new(file);
        writer.write_all(content.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Error writing: {}", e);
            std::process::exit(1);
        });
    } else {
        print!("{}", content);
    }
}
