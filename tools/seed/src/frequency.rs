use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// Parse a Yomitan frequency dictionary ZIP file.
///
/// Returns a map from term to frequency rank (lower = more common).
/// If a term appears multiple times, keeps the lowest (best) rank.
///
/// Yomitan frequency dictionaries contain `term_meta_bank_*.json` files.
/// Each entry is: `[term, "freq", frequency_data]`
/// where frequency_data can be:
/// - Simple number: `1234`
/// - Object: `{"value": 1234}` or `{"frequency": 1234}`
/// - With reading: `{"reading": "にほん", "frequency": 42}`
pub fn parse_frequency(path: &Path) -> Result<HashMap<String, u32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    let mut frequencies: HashMap<String, u32> = HashMap::new();

    // Find and parse all term_meta_bank_*.json files
    let meta_bank_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with("term_meta_bank_") && name.ends_with(".json") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    for name in meta_bank_names {
        let mut file = archive.by_name(&name)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let bank: Vec<Value> = serde_json::from_str(&contents)?;

        for entry in bank {
            if let Some((term, rank)) = parse_frequency_entry(&entry) {
                // Keep the lowest (best) rank for each term
                frequencies
                    .entry(term)
                    .and_modify(|existing| {
                        if rank < *existing {
                            *existing = rank;
                        }
                    })
                    .or_insert(rank);
            }
        }
    }

    Ok(frequencies)
}

/// Parse a single frequency entry from the JSON array format
fn parse_frequency_entry(entry: &Value) -> Option<(String, u32)> {
    let arr = entry.as_array()?;
    if arr.len() < 3 {
        return None;
    }

    let term = arr[0].as_str()?.to_string();

    // Check that this is a frequency entry (second element is "freq")
    let entry_type = arr[1].as_str()?;
    if entry_type != "freq" {
        return None;
    }

    let rank = extract_frequency_value(&arr[2])?;

    Some((term, rank))
}

/// Extract the numeric frequency value from various formats
fn extract_frequency_value(value: &Value) -> Option<u32> {
    match value {
        // Simple number
        Value::Number(n) => n.as_u64().map(|v| v as u32),

        // Object with various field names
        Value::Object(obj) => {
            // Try common field names
            for field in ["value", "frequency", "rank"] {
                if let Some(Value::Number(n)) = obj.get(field) {
                    return n.as_u64().map(|v| v as u32);
                }
            }

            // Handle nested frequency object (e.g., {"frequency": {"value": 123}})
            if let Some(freq_obj) = obj.get("frequency") {
                if let Some(n) = freq_obj.as_u64() {
                    return Some(n as u32);
                }
                if let Some(inner) = freq_obj.as_object() {
                    if let Some(Value::Number(n)) = inner.get("value") {
                        return n.as_u64().map(|v| v as u32);
                    }
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
    fn test_parse_frequency_entry_simple_number() {
        let entry = serde_json::json!(["日本", "freq", 42]);
        let (term, rank) = parse_frequency_entry(&entry).unwrap();
        assert_eq!(term, "日本");
        assert_eq!(rank, 42);
    }

    #[test]
    fn test_parse_frequency_entry_value_object() {
        let entry = serde_json::json!(["学校", "freq", {"value": 156}]);
        let (term, rank) = parse_frequency_entry(&entry).unwrap();
        assert_eq!(term, "学校");
        assert_eq!(rank, 156);
    }

    #[test]
    fn test_parse_frequency_entry_frequency_field() {
        let entry = serde_json::json!(["日本", "freq", {"reading": "にほん", "frequency": 42}]);
        let (term, rank) = parse_frequency_entry(&entry).unwrap();
        assert_eq!(term, "日本");
        assert_eq!(rank, 42);
    }

    #[test]
    fn test_parse_frequency_entry_ignores_non_freq() {
        let entry = serde_json::json!(["日本", "pitch", [0, 1, 2]]);
        assert!(parse_frequency_entry(&entry).is_none());
    }

    #[test]
    fn test_extract_frequency_value_number() {
        let value = serde_json::json!(123);
        assert_eq!(extract_frequency_value(&value), Some(123));
    }

    #[test]
    fn test_extract_frequency_value_object() {
        let value = serde_json::json!({"value": 456});
        assert_eq!(extract_frequency_value(&value), Some(456));
    }
}
