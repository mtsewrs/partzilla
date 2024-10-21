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
  pub fn new(content_type: &str) -> Result<Self> {
    if !content_type.starts_with("multipart/") {
      return Err(Error::from_reason("Invalid content type"));
    }

    let boundary = content_type
      .split(';')
      .filter_map(|part| {
        let mut split = part.trim().splitn(2, '=');
        if let (Some(key), Some(value)) = (split.next(), split.next()) {
          if key.trim() == "boundary" {
            return Some(value.trim_matches('"').to_string());
          }
        }
        None
      })
      .next();

    if let Some(boundary) = boundary {
      let mut prepended_boundary = BytesMut::with_capacity(boundary.len() + 2);
      prepended_boundary.extend_from_slice(b"--");
      prepended_boundary.extend_from_slice(boundary.as_bytes());

      Ok(MultipartParser {
        prepended_boundary: prepended_boundary.freeze(),
        remaining_body: Bytes::new(),
        first: true,
      })
    } else {
      Err(Error::from_reason("Invalid boundary"))
    }
  }

  pub fn is_valid(&self) -> bool {
    !self.prepended_boundary.is_empty()
  }

  pub fn set_body(&mut self, body: Bytes) {
    self.remaining_body = body;
  }

  pub fn get_next_part(&mut self) -> Option<(Bytes, HashMap<String, String>)> {
    if self.remaining_body.len() < self.prepended_boundary.len() {
      return None;
    }

    if self.first {
      if let Some(pos) = self
        .remaining_body
        .as_ref()
        .windows(self.prepended_boundary.len())
        .position(|window| window == self.prepended_boundary.as_ref())
      {
        self.remaining_body = self
          .remaining_body
          .slice(pos + self.prepended_boundary.len()..);
        self.first = false;
      } else {
        return None;
      }
    }

    if let Some(pos) = self
      .remaining_body
      .as_ref()
      .windows(self.prepended_boundary.len())
      .position(|window| window == self.prepended_boundary.as_ref())
    {
      let part = self.remaining_body.slice(..pos);
      self.remaining_body = self
        .remaining_body
        .slice(pos + self.prepended_boundary.len()..);

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

      if let Some(headers_end) = part
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .or_else(|| part.windows(2).position(|window| window == b"\n\n"))
      {
        let (headers_part, body_part) = part.split_at(headers_end + 4);
        let mut headers = HashMap::with_capacity(4);

        let mut part_buffer = BytesMut::from(headers_part);
        let buffer_len = part_buffer.len();
        let consumed_headers = get_headers(&mut part_buffer[..], buffer_len)?;

        for (key, value) in consumed_headers {
          headers.insert(key, value);
        }

        Some((Bytes::copy_from_slice(body_part), headers))
      } else {
        None
      }
    } else {
      None
    }
  }
}
