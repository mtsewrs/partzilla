use std::collections::HashMap;

pub fn parse_content_disposition(input: &str) -> HashMap<String, Option<String>> {
  let mut result = HashMap::with_capacity(4);

  for part in input.split(';') {
    let part = part.trim();

    if part.is_empty() {
      continue;
    }

    if let Some((key, value)) = part.split_once('=') {
      let key = key.trim().to_string();
      let value = value.trim().trim_matches('"').to_string();

      result.insert(key, if value.is_empty() { None } else { Some(value) });
    } else {
      result.insert(part.to_string(), None);
    }
  }

  result
}
