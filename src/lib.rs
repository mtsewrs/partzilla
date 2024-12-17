#![deny(clippy::all)]

mod multipart_parser;
mod utils;

#[macro_use]
extern crate napi_derive;

use napi::bindgen_prelude::*;

use multipart_parser::{MultipartParser, ParserState};
use utils::parse_content_disposition;

#[napi(object)]
pub struct MultipartFieldInfo {
  pub name: Option<String>,
  pub filename: Option<String>,
  pub content_type: Option<String>,
}

#[napi]
pub struct MultipartFileHandle {
  parser_ptr: *mut MultipartParser,
  done: bool,
}

#[napi]
impl MultipartFileHandle {
  #[napi]
  pub fn read_chunk(&mut self, size: u32) -> Option<Buffer> {
    let size: usize = size as usize;
    let parser = unsafe { &mut *self.parser_ptr };
    if self.done {
      return None;
    }

    let chunk = parser.read_body_chunk(size);
    match chunk {
      Some(c) => Some(Buffer::from(c.as_ref())),
      None => {
        if parser.is_part_ended() {
          self.done = true;
        }
        None
      }
    }
  }
}

#[napi]
pub struct MultipartParserWrapper {
  parser: Box<MultipartParser>,
}

#[napi]
impl MultipartParserWrapper {
  #[napi(constructor)]
  pub fn new(content_type: String) -> Result<Self> {
    let parser = MultipartParser::new(&content_type)?;
    Ok(MultipartParserWrapper {
      parser: Box::new(parser),
    })
  }

  #[napi]
  pub fn feed(&mut self, chunk: Buffer) {
    self.parser.feed(chunk.as_ref());
  }

  #[napi]
  pub fn end(&mut self) {
    self.parser.end_of_stream = true;
  }

  #[napi]
  pub fn get_next_file_info(&mut self) -> Option<MultipartFieldInfo> {
    self.parser.parse_until_headers();
    if let Some(headers) = &self.parser.current_headers {
      let content_disposition = headers.get("content-disposition")?;
      let parsed = parse_content_disposition(content_disposition);
      let name = parsed.get("name").cloned().flatten();
      let filename = parsed.get("filename").cloned().flatten();
      let content_type = headers.get("content-type").cloned();

      Some(MultipartFieldInfo {
        name,
        filename,
        content_type,
      })
    } else {
      None
    }
  }

  #[napi]
  pub fn start_reading_body(&mut self) -> Option<MultipartFileHandle> {
    if self.parser.state == ParserState::ReadingBody {
      Some(MultipartFileHandle {
        parser_ptr: &mut *self.parser,
        done: false,
      })
    } else {
      None
    }
  }

  #[napi]
  pub fn move_to_next_part(&mut self) {
    self.parser.move_to_next_part();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn create_parser_with_boundary(boundary: &str) -> MultipartParser {
    let content_type = format!("multipart/form-data; boundary={}", boundary);
    MultipartParser::new(&content_type).expect("Failed to create parser")
  }

  #[test]
  fn test_no_content_type() {
    let content_type = "";
    let result = MultipartParser::new(content_type);
    assert!(result.is_err(), "Expected error with empty content-type");
  }

  #[test]
  fn test_invalid_content_type() {
    let content_type = "application/json";
    let result = MultipartParser::new(content_type);
    assert!(result.is_err(), "Expected error with invalid content-type");
  }

  #[test]
  fn test_quoted_boundary() {
    let content_type = r#"multipart/form-data; boundary="myboundary""#;
    let parser = MultipartParser::new(content_type).expect("Should parse quoted boundary");
    assert_eq!(*parser.boundary, *b"--myboundary");
  }

  #[test]
  fn test_no_parts_no_end_boundary() {
    let mut parser = create_parser_with_boundary("boundary");
    let body = b"--boundary\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nValue without end boundary";
    parser.feed(body);
    parser.end_of_stream = true;

    parser.parse_until_headers();
    assert_eq!(parser.state, ParserState::ReadingBody);

    let mut collected = Vec::new();
    while let Some(chunk) = parser.read_body_chunk(10) {
      collected.extend_from_slice(&chunk);
    }
    let result_str = String::from_utf8_lossy(&collected);
    assert_eq!(result_str, "Value without end boundary");

    assert!(parser.is_part_ended());
    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }

  #[test]
  fn test_only_headers_no_body() {
    let mut parser = create_parser_with_boundary("boundary");
    let body =
      b"--boundary\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\n--boundary--\r\n";
    parser.feed(body);
    parser.end_of_stream = true;

    parser.parse_until_headers();
    let headers = parser.current_headers.as_ref().expect("No headers found");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"field\""
    );

    assert_eq!(parser.read_body_chunk(10), None);
    assert!(parser.is_part_ended());

    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }

  #[test]
  fn test_no_content_disposition() {
    let mut parser = create_parser_with_boundary("boundary");
    let body = b"--boundary\r\nContent-Type: text/plain\r\n\r\nHello\r\n--boundary--\r\n";
    parser.feed(body);
    parser.end_of_stream = true;

    parser.parse_until_headers();
    let headers = parser.current_headers.as_ref().unwrap();
    assert!(
      headers.get("content-disposition").is_none(),
      "No content-disposition header present"
    );

    assert_eq!(parser.state, ParserState::ReadingBody);

    let mut collected = Vec::new();
    while let Some(chunk) = parser.read_body_chunk(5) {
      collected.extend_from_slice(&chunk);
    }
    let body_str = String::from_utf8_lossy(&collected);
    assert_eq!(body_str, "Hello");

    assert!(parser.is_part_ended());
    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }

  #[test]
  fn test_no_content_type_for_file() {
    let mut parser = create_parser_with_boundary("boundary");
    let body = b"--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\r\nHello, file!\r\n--boundary--\r\n";
    parser.feed(body);
    parser.end_of_stream = true;

    parser.parse_until_headers();
    let headers = parser.current_headers.as_ref().expect("No headers found");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"file\"; filename=\"test.txt\""
    );

    assert_eq!(parser.state, ParserState::ReadingBody);

    let mut collected = Vec::new();
    while let Some(chunk) = parser.read_body_chunk(10) {
      collected.extend_from_slice(&chunk);
    }
    let body_str = String::from_utf8_lossy(&collected);
    assert_eq!(body_str, "Hello, file!");

    assert!(parser.is_part_ended());
    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }

  #[test]
  fn test_large_payload() {
    let mut parser = create_parser_with_boundary("boundary");
    let large_content = "X".repeat(10_000); // 10k chars
    let body = format!(
      "--boundary\r\nContent-Disposition: form-data; name=\"large\"\r\n\r\n{}\
            \r\n--boundary--\r\n",
      large_content
    );
    parser.feed(body.as_bytes());
    parser.end_of_stream = true;

    parser.parse_until_headers();
    let headers = parser.current_headers.as_ref().expect("No headers found");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"large\""
    );
    assert_eq!(parser.state, ParserState::ReadingBody);

    let mut collected = Vec::new();
    while let Some(chunk) = parser.read_body_chunk(1024) {
      collected.extend_from_slice(&chunk);
    }
    let body_str = String::from_utf8_lossy(&collected);
    assert_eq!(body_str.len(), large_content.len());
    assert_eq!(body_str, large_content);

    assert!(parser.is_part_ended());
    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }

  #[test]
  fn test_chunked_feed() {
    let mut parser = create_parser_with_boundary("boundary");
    let body = b"--boundary\r\nContent-Disposition: form-data; name=\"chunk\"\r\n\r\nchunked-data\r\n--boundary--\r\n";

    for chunk in body.chunks(5) {
      parser.feed(chunk);
    }
    parser.end_of_stream = true;

    parser.parse_until_headers();
    let headers = parser.current_headers.as_ref().expect("No headers found");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"chunk\""
    );
    assert_eq!(parser.state, ParserState::ReadingBody);

    let mut collected = Vec::new();
    while let Some(chunk) = parser.read_body_chunk(3) {
      collected.extend_from_slice(&chunk);
    }
    let body_str = String::from_utf8_lossy(&collected);
    assert_eq!(body_str, "chunked-data");

    assert!(parser.is_part_ended());
    parser.move_to_next_part();
    assert_eq!(parser.state, ParserState::Done);
  }
}
