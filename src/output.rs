use crate::error::CliError;
use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    println!("{}", to_pretty_json(value)?);
    Ok(())
}

pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, CliError> {
    serde_json::to_string_pretty(value).map_err(CliError::from)
}

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
mod tests {
    use super::to_pretty_json;
    use serde_json::json;

    #[test]
    fn to_pretty_json_preserves_object_fields() {
        let rendered = to_pretty_json(&json!({
            "message": "ok",
            "count": 2
        }))
        .expect("json should serialize");

        assert!(rendered.contains("\"message\": \"ok\""));
        assert!(rendered.contains("\"count\": 2"));
    }
}
