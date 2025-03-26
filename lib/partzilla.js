const { IncomingMessage } = require("http");
const { Readable } = require("stream");
const { Multipart, MultipartFile } = require("../index");

function partzilla(req) {
  let contentType;
  let readable;

  if (req instanceof IncomingMessage) {
    readable = Readable.toWeb(req);
    contentType = req.headers["content-type"] ?? "";
  } else if (req instanceof Request) {
    if (!req.body) {
      throw new Error("[partzilla]: No body");
    }
    readable = req.body;
    contentType = req.headers.get("content-type") ?? "";
  } else {
    throw new Error("[partzilla]: unsupported request type");
  }

  const parser = new Multipart(readable, contentType);
  return parser;
}

module.exports = { partzilla, Multipart, MultipartFile };
