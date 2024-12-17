import { test, describe, expect, afterAll, beforeAll } from "bun:test";
import { createServer } from "http";

import { partzilla } from "../lib/partzilla";

export function createRequest(url: string | URL) {
  const formData = new FormData();
  const content = '<q id="a"><span id="b">hey!</span></q>';
  const contentType = "text/html;charset=utf-8";
  const blob = new Blob([content], { type: contentType });
  formData.append("file1", blob);
  formData.append("file2", blob);
  const request = new Request(url, {
    body: formData,
    method: "POST",
  });

  return { request, content, contentType };
}

const server = createServer(async (req, res) => {
  const parser = partzilla(req);

  const files: { data: string; contentType: string }[] = [];
  for await (const file of parser.next()) {
    const data: Buffer[] = [];
    for await (const chunk of file.chunks()) {
      data.push(chunk);
    }
    files.push({
      contentType: file.content_type ?? "nope",
      data: Buffer.concat(data as unknown as Uint8Array[]).toString("utf-8"),
    });
  }

  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify(files));
});

describe("Get Parts Node stream", () => {
  beforeAll(() => {
    server.listen(8080);
  });
  afterAll(() => {
    server.close();
  });

  test("Should parse inconing multipart", async () => {
    const { request, content, contentType } = createRequest(
      "http://localhost:8080"
    );

    const response = await fetch(request);
    expect(response.ok).toBe(true);
    const json = (await response.json()) as {
      data: string;
      contentType: string;
    }[];
    expect(json.length).toBe(2);
    const field = json[0];
    expect(field.data).toEqual(content);
    expect(field.contentType).toEqual(contentType);
  });
});
