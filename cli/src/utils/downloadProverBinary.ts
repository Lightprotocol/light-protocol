import fs from "fs";
import path from "path";
import https from "https";
import http from "http";
import { pipeline } from "stream/promises";

const PROVER_VERSION = "2.0.6";
const GITHUB_RELEASES_BASE_URL = `https://github.com/Lightprotocol/light-protocol/releases/download/light-prover-v${PROVER_VERSION}`;
const MAX_REDIRECTS = 10;

interface DownloadOptions {
  maxRetries?: number;
  retryDelay?: number;
}

export async function downloadProverBinary(
  binaryPath: string,
  binaryName: string,
  options: DownloadOptions = {},
): Promise<void> {
  const { maxRetries = 3, retryDelay = 2000 } = options;
  const url = `${GITHUB_RELEASES_BASE_URL}/${binaryName}`;

  console.log(`\nDownloading prover binary: ${binaryName}`);
  console.log(`   From: ${url}`);
  console.log(`   To: ${binaryPath}\n`);

  const dir = path.dirname(binaryPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      await downloadFile(url, binaryPath);

      if (process.platform !== "win32") {
        fs.chmodSync(binaryPath, 0o755);
      }

      console.log("\nProver binary downloaded.\n");
      return;
    } catch (error) {
      lastError = error as Error;
      console.error(
        `\nDownload attempt ${attempt}/${maxRetries} failed: ${lastError.message}`,
      );

      if (attempt < maxRetries) {
        console.log(`   Retrying in ${retryDelay / 1000}s...\n`);
        await new Promise((resolve) => setTimeout(resolve, retryDelay));
      }
    }
  }

  throw new Error(
    `Failed to download prover binary after ${maxRetries} attempts: ${lastError?.message}`,
  );
}

async function downloadFile(
  url: string,
  outputPath: string,
  redirectDepth: number = 0,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith("https") ? https : http;

    const request = protocol.get(url, (response) => {
      if (
        response.statusCode === 301 ||
        response.statusCode === 302 ||
        response.statusCode === 307 ||
        response.statusCode === 308
      ) {
        const redirectUrl = response.headers.location;
        if (!redirectUrl) {
          return reject(new Error("Redirect without location header"));
        }
        if (redirectDepth >= MAX_REDIRECTS) {
          return reject(
            new Error(
              `Too many redirects: exceeded maximum of ${MAX_REDIRECTS} redirects`,
            ),
          );
        }
        return downloadFile(redirectUrl, outputPath, redirectDepth + 1).then(
          resolve,
          reject,
        );
      }

      if (response.statusCode !== 200) {
        return reject(
          new Error(`HTTP ${response.statusCode}: ${response.statusMessage}`),
        );
      }

      const totalBytes = parseInt(
        response.headers["content-length"] || "0",
        10,
      );
      let downloadedBytes = 0;
      let lastProgress = 0;

      const fileStream = fs.createWriteStream(outputPath);

      response.on("data", (chunk: Buffer) => {
        downloadedBytes += chunk.length;

        if (totalBytes > 0) {
          const progress = Math.floor((downloadedBytes / totalBytes) * 100);
          if (progress >= lastProgress + 5) {
            lastProgress = progress;
            const mb = (downloadedBytes / 1024 / 1024).toFixed(1);
            const totalMb = (totalBytes / 1024 / 1024).toFixed(1);
            process.stdout.write(
              `\r   Progress: ${progress}% (${mb}MB / ${totalMb}MB)`,
            );
          }
        }
      });

      pipeline(response, fileStream)
        .then(() => {
          if (totalBytes > 0) {
            process.stdout.write("\r   Progress: 100% - Download complete\n");
          }
          resolve();
        })
        .catch((error) => {
          fs.unlinkSync(outputPath);
          reject(error);
        });
    });

    request.on("error", (error) => {
      if (fs.existsSync(outputPath)) {
        fs.unlinkSync(outputPath);
      }
      reject(error);
    });

    request.setTimeout(60000, () => {
      request.destroy();
      reject(new Error("Download timeout"));
    });
  });
}

export function getProverVersion(): string {
  return PROVER_VERSION;
}
