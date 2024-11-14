<p background="rgb(57, 59, 58)" align="center">
  <img align="center" height="300" src="https://raw.githubusercontent.com/mtsewrs/partzilla/refs/heads/master/assets/partzilla.webp" />
</p>

<p align="center">⚡️ Multipart parser written in rust</p>
<!-- <p align="center">
  <a href="https://www.npmjs.com/~sactcore" target="_blank"><img src="https://img.shields.io/npm/v/@sact/core.svg" alt="NPM Version" /></a>
  <a href="https://www.npmjs.com/~sactcore" target="_blank"><img src="https://img.shields.io/npm/dm/@sact/core.svg" alt="NPM Downloads" /></a>
</p> -->

# Partzilla

Fast and simple multipart parser

```bash
pnpm install partzilla
bun install partzilla
yarn install partzilla
```

## Usage node

```typescript
import { getParts } from "partzilla";
import { getBodyBuffer } from "partzilla/utils";

createServer(async (req, res) => {
  const body = await getBodyBuffer(req);
  const contentType = req.headers["content-type"];
  const response = getParts(contentType, body); // const response: MultipartField[]
  res.end("Node!");
});
```

## Usage bun

```typescript
import { getParts } from "partzilla";

Bun.serve({
  async fetch(req) {
    const body = await req.arrayBuffer();
    const contentType = req.headers.get("content-type");
    const response = getParts(contentType, body); // const response: MultipartField[]
    return new Response("Bun!");
  },
});
```

## MultipartField

```typescript
interface MultipartField {
  name?: string;
  data: Buffer;
  filename?: string;
  contentType?: string;
}
```
