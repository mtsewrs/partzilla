import { type IncomingMessage } from "http";

export function getBodyBuffer(request: IncomingMessage): Promise<Buffer> {
  return new Promise((resolve) => {
    const bodyParts: Uint8Array[] = [];
    request
      .on("data", (chunk: Uint8Array) => {
        bodyParts.push(chunk);
      })
      .on("end", () => {
        resolve(Buffer.concat(bodyParts));
      });
  });
}
