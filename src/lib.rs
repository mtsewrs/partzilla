#![deny(clippy::all)]

mod multipart_parser;
mod utils;

#[macro_use]
extern crate napi_derive;

use napi::bindgen_prelude::*;

use bytes::Bytes;
use multipart_parser::MultipartParser;
use utils::parse_content_disposition;

#[napi(object)]
pub struct MultipartField {
  pub name: Option<String>,
  pub data: Buffer,
  pub filename: Option<String>,
  pub content_type: Option<String>,
}

#[napi]
pub fn get_parts(content_type: String, body: Buffer) -> Result<Vec<MultipartField>> {
  let mut parser = MultipartParser::new(content_type.as_str())?;
  if !parser.is_valid() {
    return Err(Error::from_reason("Boundry is empty"));
  }
  parser.set_body(Bytes::from(body.to_vec()));
  let mut collected = vec![];
  while let Some(part) = parser.get_next_part() {
    let headers = part.1;
    let parsed = parse_content_disposition(headers.get("content-disposition").unwrap());
    let name = parsed.get("name").cloned().flatten();
    let filename = parsed.get("filename").cloned().flatten();
    let content_type = headers.get("content-type").cloned();
    collected.push(MultipartField {
      name,
      filename,
      data: part.0.to_vec().into(),
      content_type,
    });
  }

  Ok(collected)
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;

  #[test]
  fn test_multipart_parser_with_quoted_boundary() {
    let content_type = r#"multipart/form-data; boundary="----WebKitFormBoundary""#;
    let parser = MultipartParser::new(content_type).unwrap();

    assert_eq!(
      parser.prepended_boundary,
      b"------WebKitFormBoundary".as_ref()
    );
  }

  #[test]
  fn test_multipart_parser_with_unquoted_boundary() {
    let content_type = "multipart/form-data; boundary=----WebKitFormBoundary";
    let parser = MultipartParser::new(content_type).unwrap();

    assert_eq!(
      parser.prepended_boundary,
      b"------WebKitFormBoundary".as_ref()
    );
  }

  #[test]
  fn test_multipart_parser_single_part() {
    let content_type = "multipart/form-data; boundary=----WebKitFormBoundary";
    let body = b"\r\n------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"field\"; filename=\"\"\r\nContent-Type: text/plain\r\n\r\nHello, world!\r\n------WebKitFormBoundary--\r\n".to_vec();

    let mut parser = MultipartParser::new(content_type).unwrap();
    parser.set_body(Bytes::from(body));

    let (body_part, headers) = parser.get_next_part().unwrap();

    assert_eq!(std::str::from_utf8(&body_part).unwrap(), "Hello, world!");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"field\"; filename=\"\""
    );
    assert_eq!(headers.get("content-type").unwrap(), "text/plain");
  }

  #[test]
  fn test_multipart_parser_multiple_parts() {
    let content_type = "multipart/form-data; boundary=----WebKitFormBoundary";
    let body = b"\r\n------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"field1\"\r\n\r\nValue 1\r\n------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"field2\"\r\n\r\nValue 2\r\n------WebKitFormBoundary--\r\n".to_vec();

    let mut parser = MultipartParser::new(content_type).unwrap();
    parser.set_body(Bytes::from(body));

    // First part
    let (body_part1, headers1) = parser.get_next_part().unwrap();
    assert_eq!(std::str::from_utf8(&body_part1).unwrap(), "Value 1");
    assert_eq!(
      headers1.get("content-disposition").unwrap(),
      "form-data; name=\"field1\""
    );

    // Second part
    let (body_part2, headers2) = parser.get_next_part().unwrap();
    assert_eq!(std::str::from_utf8(&body_part2).unwrap(), "Value 2");
    assert_eq!(
      headers2.get("content-disposition").unwrap(),
      "form-data; name=\"field2\""
    );
  }

  #[test]
  fn test_multipart_parser_end_boundary() {
    let content_type = "multipart/form-data; boundary=----WebKitFormBoundary";
    let body = b"\r\n------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nLast part\r\n------WebKitFormBoundary--\r\n".to_vec();

    let mut parser = MultipartParser::new(content_type).unwrap();
    parser.set_body(Bytes::from(body));

    // First and only part
    let (body_part, headers) = parser.get_next_part().unwrap();
    assert_eq!(std::str::from_utf8(&body_part).unwrap(), "Last part");
    assert_eq!(
      headers.get("content-disposition").unwrap(),
      "form-data; name=\"field\""
    );
  }
}
