//! Output formatting for human-readable and JSON display modes.
//!
//! Every command in this CLI supports two output modes:
//!
//! - **Human-readable** (default): compact, aligned text to stdout, with
//!   informational messages (totals, pagination hints) on stderr so they don't
//!   interfere with shell pipelines.
//! - **JSON** (`--json` flag): pretty-printed JSON to stdout, suitable for
//!   piping to `jq` or programmatic consumption.
//!
//! The `print_value` function handles the human-readable display of untyped
//! `serde_json::Value` responses — used for endpoints like `orders get` and
//! `feasibility status` where the full response schema is passed through
//! without a dedicated Rust type.

use crate::error::CliError;
use serde::Serialize;

/// Serialize `value` as pretty-printed JSON and print it to stdout.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    println!("{}", to_pretty_json(value)?);
    Ok(())
}

/// Serialize `value` as a pretty-printed JSON string and return it.
///
/// Used by `print_json` and in tests that need to inspect the JSON output.
pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, CliError> {
    serde_json::to_string_pretty(value).map_err(CliError::from)
}

/// Recursively print a `serde_json::Value` in a human-readable indented format.
///
/// WHY: Some API responses (notably `orders get` and `feasibility status`) do not
/// have a fixed schema and are returned as raw `serde_json::Value`. Rather than
/// dumping raw JSON in the default display mode, this function walks the value
/// tree and prints key-value pairs at the appropriate indentation level.
///
/// Null values are silently omitted to reduce noise. Arrays are printed with
/// zero-based index headers. Nested objects recurse with increased indentation.
pub fn print_value(value: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        println!("{prefix}{key}:");
                        print_value(val, indent + 1);
                    }
                    // EDGE: Null fields are skipped — they add no information and
                    // clutter the output for sparse API responses.
                    serde_json::Value::Null => {}
                    _ => {
                        println!("{prefix}{key}: {}", format_scalar(val));
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                println!("{prefix}[{i}]:");
                print_value(val, indent + 1);
            }
        }
        other => {
            println!("{prefix}{}", format_scalar(other));
        }
    }
}

/// Format a scalar JSON value as a plain string without surrounding quotes.
///
/// WHY: `serde_json`'s `Display` for strings includes the surrounding `"` characters,
/// which is correct for JSON but wrong for human-readable output.
fn format_scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "output_test.rs"]
mod tests;
