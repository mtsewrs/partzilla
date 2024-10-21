use std::collections::HashMap;

const MAX_HEADERS: usize = 10;

pub fn get_headers(post_padded_buffer: &mut [u8], end: usize) -> Option<Vec<(String, String)>> {
  let mut headers: Vec<(String, String)> = Vec::with_capacity(MAX_HEADERS);
  let mut pos = 0;

  for _ in 0..MAX_HEADERS {
    let preliminary_key_start = pos;

    // Find the key (until ':')
    while pos < end && post_padded_buffer[pos] != b':' && post_padded_buffer[pos] > 32 {
      // Lowercase conversion (same as '|= 32' for ASCII letters)
      post_padded_buffer[pos] |= 32;
      pos += 1;
    }

    if pos >= end || post_padded_buffer[pos] == b'\r' {
      if pos + 1 < end && post_padded_buffer[pos + 1] == b'\n' {
        return Some(headers); // End of headers, return collected headers
      } else {
        return None; // Invalid format
      }
    }

    // Convert key to a string
    let preliminary_key =
      std::str::from_utf8(&post_padded_buffer[preliminary_key_start..pos]).ok()?;
    pos += 1; // Skip ':'

    // Find the start of the value (skip spaces and ':')
    while pos < end
      && (post_padded_buffer[pos] == b':' || post_padded_buffer[pos] < 33)
      && post_padded_buffer[pos] != b'\r'
    {
      pos += 1;
    }
    let preliminary_value_start = pos;

    // Find the end of the value (until CRLF)
    while pos < end && post_padded_buffer[pos] != b'\r' {
      pos += 1;
    }

    if pos + 1 < end && post_padded_buffer[pos + 1] == b'\n' {
      let preliminary_value =
        std::str::from_utf8(&post_padded_buffer[preliminary_value_start..pos]).ok()?;
      headers.push((preliminary_key.to_string(), preliminary_value.to_string()));
      pos += 2; // Skip CRLF
    } else {
      return None; // Invalid format
    }
  }

  None
}

pub fn parse_content_disposition(input: &str) -> HashMap<String, Option<String>> {
  let mut result = HashMap::with_capacity(4);

  for part in input.split(';') {
    let part = part.trim();

    if part.is_empty() {
      continue;
    }

    if let Some((key, value)) = part.split_once('=') {
      let key = key.trim().to_string();
      let value = value.trim().trim_matches('"');

      result.insert(
        key,
        if value.is_empty() {
          None
        } else {
          Some(value.to_string())
        },
      );
    }
  }

  result
}
