const { Multipart, MultipartFile } = require("../index");

function partzilla(readable, contentType) {
  const parser = new Multipart(readable, contentType);
  return parser;
}

module.exports = { partzilla, Multipart, MultipartFile };
