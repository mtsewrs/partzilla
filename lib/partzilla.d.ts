import { IncomingMessage } from "http";
import { Multipart, MultipartFile } from "../index";

export function partzilla(req: IncomingMessage | Request): Multipart;

export { Multipart, MultipartFile };
