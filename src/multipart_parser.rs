use bytes::{Buf, Bytes, BytesMut};
use napi::bindgen_prelude::*;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub enum ParserState {
  SearchingBoundary,
  ReadingHeaders,
  ReadingBody,
  Done,
}

pub struct MultipartParser {
  pub boundary: Bytes,
  buffer: BytesMut,
  pub state: ParserState,
  pub current_headers: Option<HashMap<String, String>>,
  pub end_of_stream: bool,
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
        boundary: prepended_boundary.freeze(),
        buffer: BytesMut::new(),
        state: ParserState::SearchingBoundary,
        current_headers: None,
        end_of_stream: false,
      })
    } else {
      Err(Error::from_reason("Invalid boundary"))
    }
  }

  pub fn feed(&mut self, chunk: &[u8]) {
    self.buffer.extend_from_slice(chunk);
  }

  fn find_boundary(&self, start: usize) -> Option<usize> {
    self.buffer[start..]
      .windows(self.boundary.len())
      .position(|window| window == self.boundary.as_ref())
      .map(|pos| pos + start)
  }

  pub fn parse_until_headers(&mut self) {
    loop {
      match self.state {
        ParserState::SearchingBoundary => {
          if let Some(pos) = self.find_boundary(0) {
            let after = pos + self.boundary.len();
            let mut adv = after;
            if after + 2 <= self.buffer.len() && &self.buffer[after..after + 2] == b"\r\n" {
              adv += 2;
            }
            self.buffer.advance(adv);
            self.state = ParserState::ReadingHeaders;
          } else {
            if self.end_of_stream {
              self.state = ParserState::Done;
            }
            return;
          }
        }
        ParserState::ReadingHeaders => {
          if let Some(headers_end) = find_headers_end(&self.buffer) {
            let headers_part = self.buffer.split_to(headers_end);
            self.buffer.advance(4);

            if let Some(hs) = parse_headers(&headers_part) {
              self.current_headers = Some(hs);
              self.state = ParserState::ReadingBody;
              return;
            } else {
              if self.end_of_stream {
                self.state = ParserState::Done;
              }
              return;
            }
          } else {
            if self.end_of_stream {
              self.state = ParserState::Done;
            }
            return;
          }
        }
        ParserState::ReadingBody | ParserState::Done => {
          return;
        }
      }
    }
  }

  pub fn read_body_chunk(&mut self, size: usize) -> Option<Bytes> {
    if self.state != ParserState::ReadingBody {
      return None;
    }

    if let Some(boundary_pos) = self.find_boundary(0) {
      let mut available = boundary_pos;
      if available >= 2 && &self.buffer[available - 2..available] == b"\r\n" {
        available -= 2;
      }

      if available == 0 {
        return None;
      }

      let chunk_size = available.min(size);
      let chunk_mut = self.buffer.split_to(chunk_size);
      Some(chunk_mut.freeze())
    } else {
      if self.end_of_stream {
        if self.buffer.is_empty() {
          return None;
        }
        let chunk_size = self.buffer.len().min(size);
        let chunk_mut = self.buffer.split_to(chunk_size);
        Some(chunk_mut.freeze())
      } else {
        if self.buffer.is_empty() {
          return None;
        }
        let chunk_size = self.buffer.len().min(size);
        let chunk_mut = self.buffer.split_to(chunk_size);
        Some(chunk_mut.freeze())
      }
    }
  }

  pub fn is_part_ended(&self) -> bool {
    self.find_boundary(0).is_some() || self.end_of_stream
  }

  pub fn move_to_next_part(&mut self) {
    if let Some(boundary_pos) = self.find_boundary(0) {
      let after = boundary_pos + self.boundary.len();
      let mut adv = after;

      if after + 2 <= self.buffer.len() && &self.buffer[after..after + 2] == b"\r\n" {
        adv += 2;
      }

      self.buffer.advance(adv);
      self.current_headers = None;
      self.state = ParserState::ReadingHeaders;
      self.parse_until_headers();
    } else {
      self.state = ParserState::Done;
    }
  }
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
  buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_headers(data: &[u8]) -> Option<HashMap<String, String>> {
  let mut headers = HashMap::new();
  let text = std::str::from_utf8(data).ok()?;
  for line in text.split("\r\n") {
    if line.is_empty() {
      continue;
    }
    let mut parts = line.splitn(2, ':');
    let key = parts.next()?.trim().to_lowercase();
    let value = parts.next()?.trim().to_string();
    headers.insert(key, value);
  }
  Some(headers)
}
