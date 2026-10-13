//! Enumerate USB iPhones and refuse a run that cannot proceed unattended.
//!
//! Usage:
//!   cargo run --example device_preflight
//!   cargo run --example device_preflight -- --json
//!   cargo run --example device_preflight -- --require-generation legacy
//!   cargo run --example device_preflight -- --require-all-generations
//!
//! Every usbmuxd and Lockdown call is bounded. The process never prompts.
//! Exit 0 when every attached USB phone is paired, trusted, and unlocked.
//! Missing generations are warnings unless `--require-generation` (or
//! `IDEVICE_TEST_GENERATION`) / `--require-all-generations` is set. Exit 1
//! when a phone is locked, untrusted, or a required generation is absent.
//! Exit 2 when usbmuxd itself cannot be reached.

use idevice_desktop_lib::{
    device_rig::{
        IDEVICE_TEST_GENERATION, format_human_report, inspect_attached_devices,
        requested_generation, require_generations,
    },
    device_version::DeveloperGeneration,
};

fn print_usage() {
    eprintln!(
        "usage: device_preflight [--json] [--json-file PATH] [--github-output [PATH]] [--require-all-generations] [--require-generation SLUG]..."
    );
}

struct Options {
    json: bool,
    json_file: Option<String>,
    github_output: Option<String>,
    require_all: bool,
    required: Vec<DeveloperGeneration>,
}

fn next_value(
    args: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
    flag: &str,
) -> Result<String, String> {
    match args.peek() {
        Some(value) if !value.starts_with('-') => Ok(args.next().expect("peeked")),
        _ => Err(format!("{flag} needs a value")),
    }
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        json_file: None,
        github_output: None,
        require_all: false,
        required: Vec::new(),
    };
    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => options.json = true,
            "--require-all-generations" => options.require_all = true,
            "--json-file" => options.json_file = Some(next_value(&mut args, "--json-file")?),
            "--github-output" => {
                let path = match args.peek() {
                    Some(value) if !value.starts_with('-') => args.next().expect("peeked"),
                    _ => std::env::var("GITHUB_OUTPUT").unwrap_or_default(),
                };
                if path.is_empty() {
                    return Err(
                        "--github-output needs a path or GITHUB_OUTPUT in the environment".into(),
                    );
                }
                options.github_output = Some(path);
            }
            "--require-generation" => {
                let slug = next_value(&mut args, "--require-generation")?;
                let generation = DeveloperGeneration::from_slug(&slug).ok_or_else(|| {
                    format!("unknown generation {slug}; use legacy, core-device-remote, or core-device-lockdown")
                })?;
                options.required.push(generation);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.require_all {
        options.required = DeveloperGeneration::ALL.to_vec();
    } else if options.required.is_empty()
        && let Some(generation) = requested_generation()
    {
        options.required.push(generation);
    }
    Ok(options)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let options = match parse_args(std::env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            print_usage();
            std::process::exit(2);
        }
    };

    let report = match inspect_attached_devices().await {
        Ok(report) => report,
        Err(error) => {
            eprintln!("preflight: cannot talk to usbmuxd: {error}");
            std::process::exit(2);
        }
    };

    let human = format_human_report(&report);
    let json = match serde_json::to_string_pretty(&report) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("preflight: failed to serialize JSON: {error}");
            std::process::exit(2);
        }
    };
    if options.json {
        eprintln!("{human}");
        println!("{json}");
    } else {
        println!("{human}");
    }
    if let Some(path) = options.json_file
        && let Err(error) = std::fs::write(&path, &json)
    {
        eprintln!("preflight: cannot write {path}: {error}");
        std::process::exit(2);
    }

    if let Some(path) = options.github_output {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(error) = report.write_github_output(&mut file) {
                    eprintln!("preflight: writing GitHub output failed: {error}");
                    std::process::exit(2);
                }
            }
            Err(error) => {
                eprintln!("preflight: cannot open {path}: {error}");
                std::process::exit(2);
            }
        }
    }

    let mut exit = if report.ok { 0 } else { 1 };
    if let Err(error) = require_generations(&report, &options.required) {
        eprintln!("{error}");
        if options.required.len() == 1 {
            eprintln!(
                "(set {IDEVICE_TEST_GENERATION} or pass --require-generation only when that phone must be present)"
            );
        }
        exit = 1;
    }
    if exit != 0 {
        if !report.ok {
            eprintln!(
                "preflight failed: a connected device is missing, locked, untrusted, or unread"
            );
        }
        std::process::exit(exit);
    }
}
