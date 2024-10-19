import { test, describe, expect, afterAll, beforeAll } from "bun:test";
import { createServer } from "http";

import { createRequest } from "./utils/request";
import { getBodyBuffer } from "../lib/read";
import { getParts } from "../index";

const server = createServer(async (req, res) => {
  const body = await getBodyBuffer(req);
  const contentType = req.headers["content-type"] ?? "";
  const response = getParts(contentType, body);
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(
    JSON.stringify(
      response
        ? response?.map((field) => ({
            ...field,
            data: field.data.toString("utf-8"),
          }))
        : []
    )
  );
});

describe("Get Parts Node", () => {
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
    expect(json.length).toBeGreaterThan(0);
    for (const field of json) {
      expect(field.data).toEqual(content);
      expect(field.contentType).toEqual(contentType);
    }
  });
});
