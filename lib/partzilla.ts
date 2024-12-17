import { IncomingMessage } from "http";
import { MultipartParserWrapper } from "../index";

interface FileInfo {
  name?: string;
  filename?: string;
  contentType?: string;
}

type FileResult = { type: "file"; info: FileInfo } | { type: "done" };

type FileWaiter = (value: FileResult) => void;

export function partzilla(req: IncomingMessage | Request) {
  let contentType = "";
  if (req instanceof IncomingMessage) {
    contentType = req.headers["content-type"] || "";
  } else {
    contentType = req.headers.get("content-type") ?? "";
  }

  const parser = new MultipartParserWrapper(contentType);

  let feedingDone = false;
  let fileWaiters: FileWaiter[] = [];

  let feeding: Promise<void>;
  if (req instanceof IncomingMessage) {
    feeding = (async () => {
      try {
        for await (const chunk of req) {
          parser.feed(chunk);
          maybeResolveWaiters();
        }
      } finally {
        parser.end();
        feedingDone = true;
        maybeResolveWaiters();
      }
    })();
  } else {
    feeding = (async () => {
      try {
        const body = req.body; // ReadableStream<Uint8Array> | null
        if (body) {
          for await (const chunk of body) {
            // chunk is a Uint8Array
            parser.feed(Buffer.from(chunk));
            maybeResolveWaiters();
          }
        }
      } finally {
        // Signal no more data
        parser.end();
        feedingDone = true;
        maybeResolveWaiters();
      }
    })();
  }

  function maybeResolveWaiters(): void {
    if (fileWaiters.length === 0) return;

    const info = parser.getNextFileInfo();
    if (info) {
      const w = fileWaiters.shift();
      if (w) w({ type: "file", info });
      return;
    }

    if (feedingDone) {
      while (fileWaiters.length > 0) {
        const w = fileWaiters.shift();
        if (w) w({ type: "done" });
      }
    }
  }

  async function getNextFileAsync(): Promise<FileResult> {
    const info = parser.getNextFileInfo();
    if (info) {
      return { type: "file", info };
    }
    if (feedingDone) {
      return { type: "done" };
    }

    return new Promise<FileResult>((resolve) => {
      fileWaiters.push(resolve);
    });
  }

  return {
    async *next() {
      while (true) {
        const result = await getNextFileAsync();

        if (result.type === "done") {
          await feeding;
          return;
        }

        const info = result.info;
        if (!info) {
          throw new Error("Received a 'file' event without info.");
        }

        const fileHandle = parser.startReadingBody();
        if (!fileHandle) {
          await feeding;
          if (feedingDone) return;
          return;
        }

        const file = {
          name: info.name,
          filename: info.filename,
          content_type: info.contentType,
          async *chunks() {
            while (true) {
              const chunk = fileHandle.readChunk(8192);
              if (!chunk) {
                if (feedingDone) break;
                break;
              }
              yield chunk;
            }
          },
        };

        yield file;
        parser.moveToNextPart();
      }
    },
  };
}
