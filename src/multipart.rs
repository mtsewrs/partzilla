#![deny(clippy::all)]

use bytes::Bytes;
use napi::bindgen_prelude::*;
use napi::tokio::sync::mpsc::{self, Receiver, Sender};
use napi::tokio_stream::wrappers::ReceiverStream;
use std::collections::HashMap;
use std::str;

#[napi]
pub struct MultipartFile {
  pub name: Option<String>,
  pub filename: Option<String>,
  pub content_type: Option<String>,
  content: Option<Receiver<Result<Bytes>>>,
}

#[napi]
impl MultipartFile {
  fn new(headers: HashMap<String, String>) -> Self {
    let mut part = MultipartFile {
      name: None,
      filename: None,
      content_type: None,
      content: None,
    };

    if let Some(disposition) = headers.get("Content-Disposition") {
      for param in disposition.split(';') {
        let param = param.trim();
        if param.starts_with("name=") {
          let val = param
            .trim_start_matches("name=")
            .trim_matches('"')
            .to_string();
          part.name = Some(val);
        } else if param.starts_with("filename=") {
          let val = param
            .trim_start_matches("filename=")
            .trim_matches('"')
            .to_string();
          part.filename = Some(val);
        }
      }
    }

    if let Some(ct) = headers.get("Content-Type") {
      part.content_type = Some(ct.clone());
    }
    part
  }

  #[napi]
  pub fn stream(&mut self, env: &Env) -> Result<ReadableStream<BufferSlice>> {
    if let Some(stream) = self.content.take() {
      Ok(ReadableStream::create_with_stream_bytes(
        env,
        ReceiverStream::new(stream),
      )?)
    } else {
      Err(Error::from_reason("Stream already read"))
    }
  }
}

#[derive(Debug, PartialEq)]
enum ParserState {
  Preamble,
  Headers,
  Body,
  End,
}

pub struct MultipartParser {
  boundary: Vec<u8>,
  boundary_marker: Vec<u8>,
  buffer: Vec<u8>,
  state: ParserState,
  current_part: Option<MultipartFile>,
  current_sender: Option<Sender<Result<Bytes>>>,
}

impl MultipartParser {
  pub fn new(boundary: &str) -> Self {
    let b = format!("--{}", boundary);
    let bm = format!("\r\n{}", b);
    MultipartParser {
      boundary: b.into_bytes(),
      boundary_marker: bm.into_bytes(),
      buffer: Vec::new(),
      state: ParserState::Preamble,
      current_part: None,
      current_sender: None,
    }
  }

  pub async fn feed(&mut self, data: &[u8]) -> Option<MultipartFile> {
    self.buffer.extend_from_slice(data);
    loop {
      match self.state {
        ParserState::Preamble => {
          if let Some(idx) = find_subsequence(&self.buffer, &self.boundary) {
            if let Some(line_end) = find_subsequence(&self.buffer[idx..], b"\r\n") {
              let consume = idx + line_end + 2;
              self.buffer.drain(0..consume);
              self.state = ParserState::Headers;
            } else {
              break;
            }
          } else {
            break;
          }
        }
        ParserState::Headers => {
          if let Some(idx) = find_subsequence(&self.buffer, b"\r\n\r\n") {
            let headers_bytes = self.buffer[..idx].to_vec();
            self.buffer.drain(0..idx + 4);
            let headers = parse_headers(&headers_bytes);
            let mut part = MultipartFile::new(headers);
            let (tx, rx) = mpsc::channel(8);
            part.content = Some(rx);
            self.current_sender = Some(tx);
            self.current_part = Some(part);
            self.state = ParserState::Body;
          } else {
            break;
          }
        }
        ParserState::Body => {
          if let Some(idx) = find_subsequence(&self.buffer, &self.boundary_marker) {
            if idx > 0 {
              let chunk = &self.buffer[..idx];
              if let Some(sender) = &self.current_sender {
                let _ = sender
                  .send(Ok(Bytes::copy_from_slice(chunk)))
                  .await
                  .unwrap();
              }
            }
            self.buffer.drain(0..idx + self.boundary_marker.len());
            if self.buffer.starts_with(b"--") {
              self.state = ParserState::End;
            } else {
              if self.buffer.starts_with(b"\r\n") {
                self.buffer.drain(0..2);
              }
              self.current_sender.take();
              let completed_part = self.current_part.take().unwrap();
              self.state = ParserState::Headers;
              return Some(completed_part);
            }
          } else {
            let safe_len = self.boundary_marker.len();
            if self.buffer.len() > safe_len {
              let split_point = self.buffer.len() - safe_len;
              let chunk = &self.buffer[..split_point];
              if let Some(sender) = &self.current_sender {
                if !chunk.is_empty() {
                  let _ = sender.send(Ok(Bytes::copy_from_slice(chunk)));
                }
              }
              self.buffer.drain(0..split_point);
            }
            break;
          }
        }
        ParserState::End => {
          self.current_sender.take();
          if let Some(part) = self.current_part.take() {
            return Some(part);
          }
          break;
        }
      }
    }
    None
  }

  pub fn finalize(&mut self) -> Option<MultipartFile> {
    self.current_sender.take();
    self.current_part.take()
  }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  haystack
    .windows(needle.len())
    .position(|window| window == needle)
}

fn parse_headers(data: &[u8]) -> HashMap<String, String> {
  let mut headers = HashMap::new();
  if let Ok(s) = str::from_utf8(data) {
    for line in s.split("\r\n") {
      if let Some(pos) = line.find(':') {
        let key = line[..pos].trim().to_string();
        let value = line[pos + 1..].trim().to_string();
        headers.insert(key, value);
      }
    }
  }
  headers
}
