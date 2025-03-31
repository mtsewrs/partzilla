import { describe, it, expect } from "bun:test";
import { partzilla, MultipartFile } from "../lib/partzilla";

async function streamToString(stream: ReadableStream<Buffer>): Promise<string> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let result = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    result += decoder.decode(value, { stream: true });
  }
  return result;
}

describe("Multipart parser", () => {
  it("parse multipart content correctly", async () => {
    const boundary = "boundary123";
    const contentType = `multipart/form-data; boundary="${boundary}"`;

    const part1 =
      `--${boundary}\r\n` +
      `Content-Disposition: form-data; name="text"\r\n\r\n` +
      `Hello World\r\n`;
    const part2 =
      `--${boundary}\r\n` +
      `Content-Disposition: form-data; name="file"; filename="example.txt"\r\n` +
      `Content-Type: text/plain\r\n\r\n` +
      `File content\r\n`;
    const part3 = `--${boundary}--\r\n`;

    const blob = new Blob([part1, part2, part3], { type: contentType });

    const req = new Request("http://localhost:3000", {
      method: "POST",
      headers: {
        "content-type": contentType,
        "content-length": String(blob.size),
        host: "localhost:8080",
      },
      body: blob,
    });

    const multipart = partzilla(
      req.body!,
      req.headers.get("content-type") ?? ""
    );

    const partsCollected: MultipartFile[] = [];

    await multipart.next(async (part) => {
      partsCollected.push(part);
    });

    expect(partsCollected.length).toBe(2);

    const firstPart = partsCollected[0];
    expect(firstPart.name).toBe("text");
    const firstPartText = await streamToString(firstPart.stream());
    expect(firstPartText).toBe("Hello World");

    const secondPart = partsCollected[1];
    expect(secondPart.name).toBe("file");
    expect(secondPart.filename).toBe("example.txt");
    expect(secondPart.contentType).toBe("text/plain");
    const secondPartText = await streamToString(secondPart.stream());
    expect(secondPartText).toBe("File content");
  });
});
