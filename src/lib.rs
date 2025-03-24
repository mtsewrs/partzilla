#![deny(clippy::all)]

mod multipart;

#[macro_use]
extern crate napi_derive;

use futures::StreamExt;
use multipart::{MultipartFile, MultipartParser};
use napi::{bindgen_prelude::*, threadsafe_function::ThreadsafeFunction};

#[napi]
pub struct Multipart {
  reader: Reader<Uint8Array>,
  parser: MultipartParser,
}

#[napi]
impl Multipart {
  #[napi(constructor)]
  pub fn new(stream: ReadableStream<Uint8Array>, content_type: String) -> Result<Multipart> {
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
      let mut prepended = String::with_capacity(boundary.len() + 2);
      prepended.push_str("--");
      prepended.push_str(&boundary);

      let web_readable_stream = stream.read()?;
      Ok(Multipart {
        reader: web_readable_stream,
        parser: MultipartParser::new(&boundary),
      })
    } else {
      Err(Error::from_reason("Invalid boundary"))
    }
  }

  #[napi]
  pub async unsafe fn next(
    &mut self,
    callback: ThreadsafeFunction<MultipartFile, Promise<()>, MultipartFile, false>,
  ) -> Result<()> {
    while let Some(chunk) = self.reader.next().await {
      let chunk = chunk?;
      while let Some(part) = self.parser.feed(&chunk).await {
        callback.call_async(part).await?.await?;
      }
    }

    if let Some(part) = self.parser.finalize() {
      callback.call_async(part).await?.await?;
    }
    Ok(())
  }
}
