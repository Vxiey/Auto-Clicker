#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
use auto_clicker_core::engine::MouseButton;
#[cfg(windows)]
use auto_clicker_core::platform::windows::{ClickerConfig, PrecisionClicker};

#[cfg(windows)]
mod ui;

#[cfg(not(windows))]
fn main() {
    eprintln!("VxClick v0.1 currently targets Windows only.");
}

#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        return ui::run_gui();
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let command = args[1].as_str();
    if command == "gui" {
        return ui::run_gui();
    }
    if command != "benchmark" && command != "click" {
        return Err(format!("unknown command '{command}'"));
    }

    let cps = value_after(&args, "--cps")
        .unwrap_or("100")
        .parse::<f64>()
        .map_err(|_| "invalid --cps value".to_string())?;
    let seconds = value_after(&args, "--seconds")
        .unwrap_or("5")
        .parse::<f64>()
        .map_err(|_| "invalid --seconds value".to_string())?;
    let button = MouseButton::parse(value_after(&args, "--button").unwrap_or("left"))
        .ok_or_else(|| "--button must be left, right, middle, x1, or x2".to_string())?;

    let engine = PrecisionClicker::new()?;
    let stats = engine.run(ClickerConfig {
        cps,
        duration: Duration::from_secs_f64(seconds),
        button,
        ..ClickerConfig::default()
    })?;

    println!("Target CPS:       {:.3}", stats.target_cps);
    println!("Actual CPS:       {:.3}", stats.actual_cps);
    println!("Clicks:           {}", stats.clicks);
    println!("Elapsed:          {:.6} s", stats.elapsed_seconds);
    println!("Mean interval:    {:.3} us", stats.interval.mean_us);
    println!("P95 interval:     {:.3} us", stats.interval.p95_us);
    println!("P99 interval:     {:.3} us", stats.interval.p99_us);
    println!("Mean jitter:      {:.3} us", stats.jitter.mean_us);
    println!("P95 jitter:       {:.3} us", stats.jitter.p95_us);
    println!("P99 jitter:       {:.3} us", stats.jitter.p99_us);
    println!("Worst jitter:     {:.3} us", stats.jitter.worst_us);
    println!("Missed deadlines: {}", stats.missed_deadlines);
    Ok(())
}

#[cfg(windows)]
fn value_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

#[cfg(windows)]
fn print_help() {
    println!(
        "VxClick v0.1\n\n\
Usage:\n  VxClick\n  VxClick gui\n  VxClick benchmark --cps <value> --seconds <value> --button <left|right|middle|x1|x2>\n\n\
Examples:\n  VxClick benchmark --cps 500 --seconds 10\n  VxClick benchmark --cps 2000 --seconds 5 --button left\n\n\
Launching without arguments opens the native VxClick UI.\n\
The benchmark reports generated CPS and QPC-based timing jitter.\n\
Sub-millisecond rates intentionally use an active spin near each absolute deadline."
    );
}
