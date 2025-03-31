import { IncomingMessage } from "http";
import { Multipart, MultipartFile } from "../index";

export function partzilla(
  readable: ReadableStream,
  contentType: string
): Multipart;

export { Multipart, MultipartFile };
