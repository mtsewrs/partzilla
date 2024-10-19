use std::collections::HashMap;

use crate::utils::get_headers;
use bytes::{Bytes, BytesMut};

use napi::bindgen_prelude::*;

pub struct MultipartParser {
  pub prepended_boundary: Bytes,
  remaining_body: Bytes,
  first: bool,
}

impl MultipartParser {
  /// Creates a new MultipartParser and extracts the boundary from the `content_type`.
  pub fn new(content_type: &str) -> Result<Self> {
    // Expecting content type format "multipart/something; boundary=..."
    if !content_type.starts_with("multipart/") {
      return Err(Error::from_reason("Invalid content type"));
    }

    // Extract boundary from content type
    let parts: Vec<&str> = content_type.split(';').collect();
    let boundary = parts.iter().find_map(|part| {
      let pair: Vec<&str> = part.trim().split('=').collect();
      if pair.len() == 2 && pair[0].trim() == "boundary" {
        Some(pair[1].trim_matches('"').to_string())
      } else {
        None
      }
    });

    if let Some(boundary) = boundary {
      // Prepend the boundary with two hyphens
      let mut prepended_boundary = BytesMut::with_capacity(boundary.len() + 2);
      prepended_boundary.extend_from_slice(b"--"); // Add the two hyphens
      prepended_boundary.extend_from_slice(boundary.as_bytes()); // Append the actual boundary

      Ok(MultipartParser {
        prepended_boundary: prepended_boundary.freeze(),
        remaining_body: Bytes::new(),
        first: true,
      })
    } else {
      Err(Error::from_reason("Invalid boundry"))
    }
  }

  pub fn is_valid(&self) -> bool {
    !self.prepended_boundary.is_empty()
  }

  /// Sets the multipart body for the parser.
  pub fn set_body(&mut self, body: Bytes) {
    self.remaining_body = body;
  }

  /// Parses and extracts the next part of the body, including headers and content.
  pub fn get_next_part(&mut self) -> Option<(Bytes, HashMap<String, String>)> {
    let mut headers = HashMap::new();

    if self.remaining_body.len() < self.prepended_boundary.len() {
      return None; // Not enough data for the boundary
    }

    if self.first {
      // First boundary parsing
      let next_boundary = self
        .remaining_body
        .windows(self.prepended_boundary.len())
        .position(|window| window == self.prepended_boundary);

      if let Some(pos) = next_boundary {
        // Discard everything before and including the boundary
        self.remaining_body = self
          .remaining_body
          .slice(pos + self.prepended_boundary.len()..);
        self.first = false;
      } else {
        return None;
      }
    }

    // Find the next boundary that marks the end of the current part
    let next_end_boundary = self
      .remaining_body
      .windows(self.prepended_boundary.len())
      .position(|window| window == self.prepended_boundary);

    if let Some(pos) = next_end_boundary {
      let part = self.remaining_body.slice(..pos); // Get the part data up to the boundary
      self.remaining_body = self
        .remaining_body
        .slice(pos + self.prepended_boundary.len()..); // Move past the boundary

      // Remove \r\n at the start and end of the part (cross-platform)
      let part = if part.starts_with(b"\r\n") {
        part.slice(2..)
      } else {
        part
      };
      let part = if part.ends_with(b"\r\n") {
        part.slice(..part.len() - 2)
      } else {
        part
      };

      // Split part into headers and body (find where \r\n\r\n or \n\n separates headers from body)
      let headers_end = part
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .or_else(|| part.windows(2).position(|window| window == b"\n\n"))
        .unwrap_or(part.len());

      let (headers_part, body_part) = part.split_at(headers_end + 4); // Skip past \r\n\r\n or \n\n

      // Parse headers (e.g., Content-Disposition and Content-Type)
      let mut part_buffer = headers_part.to_vec();
      let buffer_len = part_buffer.len();
      let consumed_headers = get_headers(&mut part_buffer, buffer_len)?;

      for (key, value) in consumed_headers {
        headers.insert(key, value);
      }

      Some((body_part.to_vec().into(), headers.clone())) // Return the body as Bytes
    } else {
      None // No more parts found
    }
  }
}
