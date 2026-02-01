use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// A parsed term entry from a Yomitan dictionary
#[derive(Debug, Clone)]
pub struct TermEntry {
    /// The term (kanji or kana-only word)
    pub term: String,
    /// Hiragana reading (empty if same as term)
    pub reading: String,
    /// List of definitions
    pub definitions: Vec<String>,
    /// Whether this word is usually written in kana (JMdict "uk" tag)
    pub usually_kana: bool,
}

/// Parse a Yomitan dictionary ZIP file and extract all term entries.
///
/// Yomitan dictionaries contain `term_bank_*.json` files, each holding an array
/// of term entries. Each entry is a JSON array:
/// `[term, reading, tags, rules, score, definitions, seq, term_tags]`
pub fn parse_dictionary(path: &Path) -> Result<Vec<TermEntry>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut entries = Vec::new();

    // Find and parse all term_bank_*.json files
    let term_bank_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with("term_bank_") && name.ends_with(".json") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    for name in term_bank_names {
        let mut file = archive.by_name(&name)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let bank: Vec<Value> = serde_json::from_str(&contents)?;

        for entry in bank {
            if let Some(parsed) = parse_term_entry(&entry) {
                entries.push(parsed);
            }
        }
    }

    Ok(entries)
}

/// Parse a single term entry from the JSON array format
fn parse_term_entry(entry: &Value) -> Option<TermEntry> {
    let arr = entry.as_array()?;
    if arr.len() < 6 {
        return None;
    }

    let term = arr[0].as_str()?.to_string();
    let reading = arr[1].as_str()?.to_string();
    let tags = arr[2].as_str().unwrap_or("");
    let definitions = extract_definitions(&arr[5]);

    // Check for "uk" (usually kana) tag
    // 1. In the tags field (space-separated)
    // 2. In structured content definitions (Jitendex stores it as "code": "uk")
    let usually_kana =
        tags.split_whitespace().any(|t| t == "uk") || has_uk_in_definitions(&arr[5]);

    Some(TermEntry {
        term,
        reading,
        definitions,
        usually_kana,
    })
}

/// Check if the definitions contain a "uk" (usually kana) tag in structured content
fn has_uk_in_definitions(value: &Value) -> bool {
    match value {
        Value::String(s) => s.contains("\"code\":\"uk\"") || s.contains("\"code\": \"uk\""),
        Value::Array(arr) => arr.iter().any(has_uk_in_definitions),
        Value::Object(obj) => {
            // Check if this object has "code": "uk"
            if let Some(Value::String(code)) = obj.get("code") {
                if code == "uk" {
                    return true;
                }
            }
            // Recursively check all values
            obj.values().any(has_uk_in_definitions)
        }
        _ => false,
    }
}

/// Extract plain text definitions from the definitions field.
/// Handles both simple string arrays and structured content.
fn extract_definitions(value: &Value) -> Vec<String> {
    let mut definitions = Vec::new();

    let Some(arr) = value.as_array() else {
        return definitions;
    };

    for item in arr {
        if let Some(s) = item.as_str() {
            // Simple string definition
            definitions.push(s.to_string());
        } else if let Some(obj) = item.as_object() {
            // Structured content - extract text recursively
            if let Some(text) = flatten_structured_content(item) {
                if !text.is_empty() {
                    definitions.push(text);
                }
            } else if let Some(glossary) = obj.get("glossary") {
                // Some formats nest glossary inside
                definitions.extend(extract_definitions(glossary));
            }
        }
    }

    definitions
}

/// Flatten structured content to plain text by recursively extracting text nodes
fn flatten_structured_content(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().filter_map(flatten_structured_content).collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(""))
            }
        }
        Value::Object(obj) => {
            // Handle "content" field in structured content
            if let Some(content) = obj.get("content") {
                return flatten_structured_content(content);
            }
            // Handle "text" field
            if let Some(text) = obj.get("text") {
                return flatten_structured_content(text);
            }
            // Check for tag-based content
            if obj.contains_key("tag") {
                if let Some(content) = obj.get("content") {
                    return flatten_structured_content(content);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_term_entry_simple() {
        let entry = serde_json::json!([
            "日本",
            "にほん",
            "",
            "",
            100,
            ["Japan"],
            1,
            ""
        ]);

        let parsed = parse_term_entry(&entry).unwrap();
        assert_eq!(parsed.term, "日本");
        assert_eq!(parsed.reading, "にほん");
        assert_eq!(parsed.definitions, vec!["Japan"]);
        assert!(!parsed.usually_kana);
    }

    #[test]
    fn test_parse_term_entry_multiple_definitions() {
        let entry = serde_json::json!([
            "学校",
            "がっこう",
            "",
            "",
            50,
            ["school", "educational institution"],
            2,
            ""
        ]);

        let parsed = parse_term_entry(&entry).unwrap();
        assert_eq!(parsed.term, "学校");
        assert_eq!(parsed.reading, "がっこう");
        assert_eq!(parsed.definitions, vec!["school", "educational institution"]);
    }

    #[test]
    fn test_extract_definitions_strings() {
        let value = serde_json::json!(["definition one", "definition two"]);
        let defs = extract_definitions(&value);
        assert_eq!(defs, vec!["definition one", "definition two"]);
    }

    #[test]
    fn test_flatten_structured_content() {
        let content = serde_json::json!({
            "tag": "span",
            "content": "Hello world"
        });
        assert_eq!(flatten_structured_content(&content), Some("Hello world".to_string()));
    }

    #[test]
    fn test_flatten_nested_structured_content() {
        let content = serde_json::json!({
            "tag": "div",
            "content": [
                "Hello ",
                { "tag": "b", "content": "world" }
            ]
        });
        assert_eq!(flatten_structured_content(&content), Some("Hello world".to_string()));
    }

    #[test]
    fn test_usually_kana_tag_detected() {
        let entry = serde_json::json!([
            "為る",
            "する",
            "uk v1",
            "",
            100,
            ["to do"],
            1,
            ""
        ]);

        let parsed = parse_term_entry(&entry).unwrap();
        assert_eq!(parsed.term, "為る");
        assert!(parsed.usually_kana);
    }
}
