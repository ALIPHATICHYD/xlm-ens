use clap::ValueEnum;
use colored::{control, Colorize};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::future::Future;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static COLOR_ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    #[value(name = "csv")]
    Csv,
}

pub fn configure(no_color: bool) {
    let enabled = !no_color && std::env::var_os("NO_COLOR").is_none();
    COLOR_ENABLED.store(enabled, Ordering::Relaxed);
    control::set_override(enabled);
}

pub fn color_enabled() -> bool {
    COLOR_ENABLED.load(Ordering::Relaxed)
}

pub fn should_show_progress(format: OutputFormat) -> bool {
    format == OutputFormat::Human && color_enabled() && std::io::stderr().is_terminal()
}

fn stylize_line(line: &str) -> String {
    if !color_enabled() {
        return line.to_string();
    }

    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with("SUCCESS:") || trimmed.starts_with("  - SUCCESS:") {
        return line.green().bold().to_string();
    }
    if trimmed.starts_with("ERROR:") || trimmed.starts_with("  - ERROR:") || trimmed.starts_with("Failed ")
    {
        return line.red().bold().to_string();
    }
    if trimmed.starts_with("Warning:") || trimmed.contains("timed out") || trimmed.contains("retrying") {
        return line.yellow().bold().to_string();
    }
    if trimmed.starts_with("[PASS]")
        || trimmed.starts_with("    [PASS]")
        || trimmed.starts_with("[OK]")
        || trimmed.starts_with("    [OK]")
        || trimmed.contains(" PASS ")
    {
        return line.green().bold().to_string();
    }
    if trimmed.starts_with("[FAIL]")
        || trimmed.starts_with("    [FAIL]")
        || trimmed.starts_with("DEGRADED")
        || trimmed.contains(" FAIL ")
    {
        return line.red().bold().to_string();
    }
    if trimmed.contains(':') && !trimmed.contains("://") {
        return line.cyan().to_string();
    }
    if !line.starts_with("  ") && !trimmed.contains(':') {
        return line.cyan().bold().to_string();
    }
    if trimmed.starts_with("(This is a read-only")
        || trimmed.starts_with("Dry run:")
        || trimmed.starts_with("RPC ")
    {
        return line.cyan().to_string();
    }

    line.to_string()
}

pub fn stylize_block(text: &str) -> String {
    text.lines().map(stylize_line).collect::<Vec<_>>().join("\n")
}

pub fn print_human(text: &str) {
    println!("{}", stylize_block(text));
}

pub fn print_human_err(text: &str) {
    eprintln!("{}", stylize_block(text));
}

pub fn spinner(message: impl Into<String>, format: OutputFormat) -> Option<ProgressBar> {
    if !should_show_progress(format) {
        return None;
    }

    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("valid spinner template")
            .tick_strings(&["|", "/", "-", "\\"]),
    );
    bar.set_message(message.into());
    bar.enable_steady_tick(std::time::Duration::from_millis(100));
    Some(bar)
}

pub fn progress_bar(
    total: u64,
    message: impl Into<String>,
    format: OutputFormat,
) -> Option<ProgressBar> {
    if !should_show_progress(format) {
        return None;
    }

    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .expect("valid progress template")
            .progress_chars("=>-"),
    );
    bar.set_message(message.into());
    Some(bar)
}

pub fn finish_spinner(bar: Option<ProgressBar>, message: impl Into<String>) {
    if let Some(bar) = bar {
        bar.finish_with_message(message.into());
    }
}

pub fn abandon_spinner(bar: Option<ProgressBar>, message: impl Into<String>) {
    if let Some(bar) = bar {
        bar.abandon_with_message(message.into());
    }
}

pub async fn with_spinner<T, E, Fut>(
    message: impl Into<String>,
    format: OutputFormat,
    fut: Fut,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    let message = message.into();
    let bar = spinner(message.clone(), format);
    match fut.await {
        Ok(value) => {
            finish_spinner(bar, format!("{} done", message));
            Ok(value)
        }
        Err(err) => {
            abandon_spinner(bar, format!("{} failed", message));
            Err(err)
        }
    }
}

pub fn emit(format: OutputFormat, human: &str, json: Value) {
    match format {
        OutputFormat::Human => print_human(human),
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json).expect("json output should always serialize")
            );
        }
        OutputFormat::Csv => {
            emit_csv(&json, &mut std::io::stdout());
        }
    }
}

pub fn emit_error(format: OutputFormat, human: &str, json: Value) {
    match format {
        OutputFormat::Human => print_human_err(human),
        OutputFormat::Json => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json).expect("json output should always serialize")
            );
        }
        OutputFormat::Csv => {
            emit_csv(&json, &mut std::io::stderr());
        }
    }
}

fn emit_csv(json: &Value, writer: &mut dyn std::io::Write) {
    if let Value::Object(map) = json {
        let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
        let values: Vec<String> = map
            .values()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        let _ = writeln!(writer, "{}", keys.join(","));
        let _ = writeln!(writer, "{}", values.join(","));
    }
}

#[cfg(test)]
mod tests {
    use super::OutputFormat;
    use clap::ValueEnum;

    #[test]
    fn test_invalid_output_mode_exits_with_error() {
        assert!(OutputFormat::from_str("tsv", false).is_err());
    }
}
