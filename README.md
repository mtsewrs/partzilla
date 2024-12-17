<p background="rgb(57, 59, 58)" align="center">
  <img align="center" height="300" src="https://raw.githubusercontent.com/mtsewrs/partzilla/refs/heads/master/assets/partzilla.webp" />
</p>

<h2 align="center">⚡️ Modern multipart parser written in rust and typescript for node and bun</h2>
<p align="center">
  <a href="https://www.npmjs.com/~partzilla" target="_blank"><img src="https://img.shields.io/npm/v/partzilla.svg" alt="NPM Version" /></a>
  <a href="https://www.npmjs.com/~partzilla" target="_blank"><img src="https://img.shields.io/npm/dm/partzilla.svg" alt="NPM Downloads" /></a>
</p>

## Install

```bash
pnpm install partzilla
bun install partzilla
yarn install partzilla
npm install partzilla
```

## Usage node

```typescript
import { partzilla } from "partzilla";

createServer(async (req, res) => {
  const files = partzilla(req);

  for await (const file of files.next()) {
    console.log(file); // MultipartFile
    for await (const chunk of file.chunks()) {
      console.log(chunk); // chunk is a buffer, stream to file or whatever
    }
  }
  res.end("Node!");
});
```

## Usage bun

```typescript
import { partzilla } from "partzilla";

Bun.serve({
  async fetch(req) {
    const files = partzilla(req);

    for await (const file of files.next()) {
      console.log(file); // MultipartFile
      for await (const chunk of file.chunks()) {
        console.log(chunk); // chunk is a buffer, stream to file or whatever
      }
    }
    return new Response("Bun!");
  },
});
```

## MultipartFile

```typescript
interface MultipartFile {
  name?: string;
  filename?: string;
  content_type?: string;
  chunks(): AsyncGenerator<Buffer, void, unknown>;
}
```
