mod config;
mod formatter;
mod models;
mod providers;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use config::{default_config, default_config_path, read_config, write_config};
use formatter::EventFormatter;
use providers::create_provider;

#[derive(Parser)]
#[command(name = "agenda", about = "Fetch calendar events as markdown")]
struct Cli {
    #[arg(long, help = "Path to configuration file")]
    config: Option<PathBuf>,

    #[arg(long, help = "Override the provider from config")]
    provider: Option<String>,

    #[arg(
        long,
        help = "Override the time format from config (strftime, e.g. %H:%M)"
    )]
    time_format: Option<String>,

    #[arg(long, help = "Override the event template from config")]
    event_template: Option<String>,

    #[arg(long, short, help = "Enable verbose logging")]
    verbose: bool,

    #[arg(
        long,
        help = "Date to fetch events for (YYYY-MM-DD, default: today)",
        value_parser = parse_date
    )]
    date: Option<NaiveDate>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Initialize default configuration")]
    Init,
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("invalid date '{}', expected YYYY-MM-DD", s))
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Init) = &cli.command {
        let cfg = default_config();
        let path = default_config_path();
        write_config(&cfg, &path)?;
        println!("Created default configuration at: {}", path.display());
        println!(
            "Please set your API key in the {} environment variable.",
            cfg.providers[&cfg.provider].env_api_key
        );
        return Ok(());
    }

    let config_path = cli.config.unwrap_or_else(default_config_path);
    let mut cfg = read_config(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;

    // Apply CLI overrides
    if let Some(p) = cli.provider {
        cfg.provider = p;
    }
    if let Some(tf) = cli.time_format {
        cfg.time_format = tf;
    }
    if let Some(et) = cli.event_template {
        cfg.event_template = et;
    }

    let date = cli.date.unwrap_or_else(|| Local::now().date_naive());

    if cli.verbose {
        eprintln!("Provider:       {}", cfg.provider);
        eprintln!("Time format:    {}", cfg.time_format);
        eprintln!("Event template: {}", cfg.event_template);
        eprintln!("Date:           {}", date);
    }

    let provider = create_provider(&cfg.provider, &cfg)?;

    // Spinner on main thread; API calls on background thread
    let style = ProgressStyle::default_spinner().tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(style);
    spinner.set_message("Fetching events...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let handle = std::thread::spawn(move || provider.get_events(date));
    let events = handle.join().expect("provider thread panicked")?;

    for event in &events {
        if cli.verbose {
            eprintln!(
                "Fetched event: ({}) {} ({} - {})",
                event.id, event.title, event.start_time, event.end_time,
            );
        }
    }

    spinner.finish_and_clear();

    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }

    // Deduplicate by event ID
    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut unique_events: Vec<_> = events
        .into_iter()
        .filter(|e| seen.insert(e.id.clone(), ()).is_none())
        .collect();

    // Sort by start time
    unique_events.sort_by_key(|e| e.start_time);

    let formatter = EventFormatter::new(&cfg.time_format, &cfg.event_template)?;
    for event in &unique_events {
        let line = formatter.format_event(event)?;
        println!("{}", line);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
