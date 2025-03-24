import { IncomingMessage } from "http";
import { Readable } from "stream";
import { Multipart, MultipartFile } from "../index";

export function partzilla(req: IncomingMessage | Request): Multipart {
  let contentType: string | undefined;

  let readable: ReadableStream;
  if (req instanceof IncomingMessage) {
    readable = Readable.toWeb(req);
    contentType = req.headers["content-type"] ?? "";
  } else {
    if (!req.body) {
      throw new Error("[partzilla]: No body");
    }
    readable = req.body;
    contentType = req.headers.get("content-type") ?? "";
  }

  const parser = new Multipart(readable, contentType);

  return parser;
}

export { Multipart, MultipartFile };
