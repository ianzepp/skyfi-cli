use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string())
    );
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
