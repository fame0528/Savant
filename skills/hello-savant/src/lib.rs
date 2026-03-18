use serde::Deserialize;
use std::fmt::Write;

#[derive(Debug, Deserialize)]
struct SkillInput {
    action: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    style: String,
}

#[no_mangle]
pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => return error_response("Invalid UTF-8 input"),
    };

    let parsed: SkillInput = match serde_json::from_str(input_str) {
        Ok(p) => p,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
    };

    let result = match parsed.action.as_str() {
        "greet" => greet(&parsed.name),
        "word_count" => word_count(&parsed.text),
        "format_text" => format_text(&parsed.text, &parsed.style),
        other => Err(format!("Unknown action: {}", other)),
    };

    match result {
        Ok(output) => success_response(&output),
        Err(e) => error_response(&e),
    }
}

fn greet(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.len() > 100 {
        return Err("Name too long (max 100 characters)".to_string());
    }
    Ok(format!("Hello, {}! Welcome to Savant.", name))
}

fn word_count(text: &str) -> Result<String, String> {
    if text.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    let word_count = text.split_whitespace().count();
    let char_count = text.chars().count();
    let line_count = text.lines().count();

    Ok(serde_json::json!({
        "word_count": word_count,
        "char_count": char_count,
        "line_count": line_count,
        "avg_word_length": if word_count > 0 {
            text.split_whitespace().map(|w| w.len()).sum::<usize>() as f64 / word_count as f64
        } else {
            0.0
        }
    })
    .to_string())
}

fn format_text(text: &str, style: &str) -> Result<String, String> {
    if text.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    match style {
        "uppercase" => Ok(text.to_uppercase()),
        "lowercase" => Ok(text.to_lowercase()),
        "titlecase" => {
            let mut result = String::with_capacity(text.len());
            let mut capitalize_next = true;
            for c in text.chars() {
                if c.is_whitespace() {
                    result.push(c);
                    capitalize_next = true;
                } else if capitalize_next {
                    write!(result, "{}", c.to_uppercase()).map_err(|e| e.to_string())?;
                    capitalize_next = false;
                } else {
                    result.push(c);
                }
            }
            Ok(result)
        }
        other => Err(format!(
            "Unknown style '{}'. Use: uppercase, lowercase, titlecase",
            other
        )),
    }
}

fn success_response(output: &str) -> *const u8 {
    let json = format!(
        "{{\"success\":true,\"output\":{}}}",
        serde_json::to_string(output).unwrap_or_default()
    );
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes);
    ptr
}

fn error_response(message: &str) -> *const u8 {
    let json = format!(
        "{{\"success\":false,\"error\":{}}}",
        serde_json::to_string(message).unwrap_or_default()
    );
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes);
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet_valid() {
        let result = greet("World");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("World"));
    }

    #[test]
    fn test_greet_empty() {
        assert!(greet("").is_err());
    }

    #[test]
    fn test_word_count() {
        let result = word_count("hello world test");
        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["word_count"], 3);
    }

    #[test]
    fn test_format_uppercase() {
        assert_eq!(format_text("hello", "uppercase").unwrap(), "HELLO");
    }

    #[test]
    fn test_format_titlecase() {
        assert_eq!(
            format_text("hello world", "titlecase").unwrap(),
            "Hello World"
        );
    }

    #[test]
    fn test_format_unknown() {
        assert!(format_text("test", "unknown").is_err());
    }
}
