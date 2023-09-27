import axios from "axios";
import * as fs from "fs";
import { promisify } from "util";
import cliProgress from "cli-progress";
import * as path from "path";
import * as tar from "tar";
import * as zlib from "zlib";
import { sleep, getSystem, System } from "@lightprotocol/zk.js";

const fileExists = promisify(fs.exists);

async function latestRelease(owner: string, repo: string) {
  const github = "https://api.github.com";
  console.log(
    `🔍 Checking the latest release of ${github}/repos/${owner}/${repo}/releases/latest`
  );

  const response = await axios.get(
    `${github}/repos/${owner}/${repo}/releases/latest`
  );
  const tag_name = response.data.tag_name;

  console.log(`📦 The newest release of ${repo} is ${tag_name}`);

  return response.data.tag_name;
}

/**
 * Makes the given file executable.
 * @param filePath - The path to the file to make executable.
 */
function makeExecutable(filePath: string): void {
  fs.chmodSync(filePath, "755");
}

/**
 * Makes all files without extensions in the given directory executable.
 * @param dirPath - The path to the directory to make files executable.
 * @returns {Promise<void>}
 */
async function makeExecutableInDir(dirPath: string): Promise<void> {
  const files = fs.readdirSync(dirPath);

  for (const file of files) {
    const filePath = path.join(dirPath, file);
    const stat = fs.statSync(filePath);
    const extname = path.extname(filePath);

    if (stat.isDirectory()) {
      await makeExecutableInDir(filePath);
    } else if (
      !filePath.startsWith(".") &&
      (extname === "" || extname === ".sh")
    ) {
      fs.chmodSync(filePath, "755");
    }
  }
}

/**
 * Decompresses the given downloaded data stream to the given local file path.
 * @param decompressor - The decompressor to use.
 * @param data - The data stream to decompress.
 * @param localFilePath - The local file path to decompress the data to. If
 * provided, that only file will be decompressed from the archive. If not
 * provided, all files will be decompressed to `dirPath`.
 * @param dirPath - The directory path to decompress the data to.
 * @returns {Promise<void>}
 */
function handleTarFile({
  decompressor,
  data,
  localFilePath,
  dirPath,
}: {
  decompressor: any;
  data: any;
  localFilePath?: string;
  dirPath: string;
}) {
  const parser = new tar.Parse();

  data.pipe(decompressor).pipe(parser);

  parser.on("entry", (entry: any) => {
    const baseName = path.parse(entry.path).base;
    const outputFilePath = localFilePath
      ? localFilePath
      : path.join(dirPath, entry.path);

    if (baseName.startsWith("._")) {
      // Ignore AppleDouble files.
      entry.resume();
    } else if (
      !localFilePath ||
      entry.path === path.parse(localFilePath).base
    ) {
      // Unpack the file if it's the one we want, or if we want all files.

      // Create directory if it does not exist.
      const dir = path.dirname(outputFilePath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      // Check if entry is a file before attempting to create a write stream for it
      if (entry.type === "file") {
        entry.pipe(fs.createWriteStream(outputFilePath));
      } else if (entry.type === "File") {
        entry.pipe(fs.createWriteStream(outputFilePath));
      } else {
        entry.resume();
      }
    } else {
      entry.resume();
    }
  });

  return new Promise<void>((resolve, _reject) => {
    parser.on("end", () => {
      // Make the file executable after it has been written.
      if (localFilePath) {
        makeExecutable(localFilePath);
      } else {
        makeExecutableInDir(dirPath);
      }
      resolve();
    });
  });
}

/**
 * Downloads a file from the given URL to the given local file path.
 * @param localFilePath - The local file path to download the file to. If
 * provided and the download file is an archive, only the file with the same
 * name as `localFilePath` will be extracted from the archive. If not provided,
 * all files will be extracted to `dirPath`.
 * @param dirPath - The path to the directory where the file(s) will be created.
 * @param url - The URL to download the file from.
 * @returns {Promise<void>}
 */
export async function downloadFile({
  localFilePath,
  dirPath,
  url,
}: {
  localFilePath?: string;
  dirPath: string;
  url: string;
}) {
  console.log(`📥 Downloading ${url}...`);
  const { data, headers } = await axios({
    url,
    method: "GET",
    responseType: "stream",
  });

  const totalLength = headers["content-length"];
  const progressBar = new cliProgress.SingleBar(
    {},
    cliProgress.Presets.shades_classic
  );
  progressBar.start(totalLength, 0);

  data.on("data", (chunk: any) => {
    progressBar.increment(chunk.length);
  });

  data.on("end", () => {
    progressBar.stop();
  });

  // If the file is a tar.gz file, decompress it while it's being written.
  if (url.endsWith(".tar.gz")) {
    console.log(`📦 Extracting ${url}...`);
    const decompressor = zlib.createGunzip();
    return handleTarFile({
      decompressor,
      data,
      localFilePath,
      dirPath,
    });
  } else {
    if (!localFilePath) throw new Error("localFilePath is undefined");
    const writeStream = fs.createWriteStream(localFilePath);
    data.pipe(writeStream);

    return new Promise<void>((resolve, reject) => {
      writeStream.on("finish", () => {
        // Make the file executable after it has been written.
        makeExecutable(localFilePath);
        resolve();
      });
      writeStream.on("error", reject);
    });
  }
}

/**
 * Download a binary from the given release artifact of the GitHub repository,
 * if it was not already downloaded.
 * @param localFilePath - The path to the local file (which either already
 * exists or will be created).
 * @param dirPath - The path to the directory where the file(s) will be created.
 * @param owner - The owner of the GitHub repository.
 * @param repoName - The name of the GitHub repository.
 * @param remoteFileName - The name of the file in the GitHub release artifact.
 * @returns {Promise<void>}
 */
export async function downloadBinIfNotExists({
  localFilePath,
  dirPath,
  owner,
  repoName,
  remoteFileName,
  tag,
}: {
  localFilePath: string;
  dirPath: string;
  owner: string;
  repoName: string;
  remoteFileName: string;
  tag?: string;
}) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }

  // Check if file exists
  if (await fileExists(localFilePath)) {
    return;
  }
  if (!tag) tag = await latestRelease(owner, repoName);
  const url = `https://github.com/${owner}/${repoName}/releases/download/${tag}/${remoteFileName}`;

  await downloadFile({
    localFilePath,
    dirPath,
    url,
  });
  // Wait for a second to make sure the file is written
  await sleep(1000);
}

function lightSystemSuffix(): string {
  let systemSuffix: string;
  switch (getSystem()) {
    case System.LinuxAmd64:
      systemSuffix = "linux-amd64";
      break;
    case System.LinuxArm64:
      systemSuffix = "linux-arm64";
      break;
    case System.MacOsAmd64:
      systemSuffix = "macos-amd64";
      break;
    case System.MacOsArm64:
      systemSuffix = "macos-arm64";
      break;
  }
  return systemSuffix;
}

/**
 * Download a binary of a Light Protocol associated project. They all share
 * common properties (e.g. the owner, the OS and CPU architecture suffix, etc.).
 * @param localFilePath - The path to the local file (which either already
 * exists or will be created).
 * @param dirPath - The path to the directory where the file will be created.
 * @param repoName - The name of the GitHub repository.
 * @param remoteFileName - The name of the file in the GitHub release artifact.
 * @returns {Promise<void>}
 */
export async function downloadLightBinIfNotExists({
  localFilePath,
  dirPath,
  repoName,
  remoteFileName,
  tag,
}: {
  localFilePath: string;
  dirPath: string;
  repoName: string;
  remoteFileName: string;
  tag?: string;
}) {
  const systemSuffix = lightSystemSuffix();
  const fullRemoteFileName = `${remoteFileName}-${systemSuffix}`;
  console.log("fullRemoteFileName", fullRemoteFileName);
  await downloadBinIfNotExists({
    localFilePath,
    dirPath,
    owner: "Lightprotocol",
    repoName,
    remoteFileName: fullRemoteFileName,
    tag,
  });
}

export async function downloadSolanaIfNotExists({
  dirPath,
}: {
  dirPath: string;
}) {
  if (fs.existsSync(dirPath)) {
    return;
  }
  fs.mkdirSync(dirPath, { recursive: true });

  const systemSuffix = lightSystemSuffix();

  const depsDirPath = path.join(dirPath, "deps");

  const tag = await latestRelease("Lightprotocol", "solana");
  const solanaUrl = `https://github.com/Lightprotocol/solana/releases/download/${tag}/solana-${systemSuffix}.tar.gz`;
  const solanaDepsUrl = `https://github.com/Lightprotocol/solana/releases/download/${tag}/solana-deps-${systemSuffix}.tar.gz`;
  const solanaSdkUrl = `https://github.com/Lightprotocol/solana/releases/download/${tag}/solana-sdk-sbf-${systemSuffix}.tar.gz`;

  await downloadFile({
    dirPath,
    url: solanaUrl,
  });
  await downloadFile({
    dirPath: depsDirPath,
    url: solanaDepsUrl,
  });
  await downloadFile({
    dirPath,
    url: solanaSdkUrl,
  });
}

export async function downloadCargoGenerateIfNotExists({
  localFilePath,
  dirPath,
}: {
  localFilePath: string;
  dirPath: string;
}) {
  const tag = await latestRelease("cargo-generate", "cargo-generate");
  let remoteFileName: string;
  switch (getSystem()) {
    case System.LinuxAmd64:
      remoteFileName = `cargo-generate-${tag}-x86_64-unknown-linux-musl.tar.gz`;
      break;
    case System.LinuxArm64:
      remoteFileName = `cargo-generate-${tag}-aarch64-unknown-linux-musl.tar.gz`;
      break;
    case System.MacOsAmd64:
      remoteFileName = `cargo-generate-${tag}-x86_64-apple-darwin.tar.gz`;
      break;
    case System.MacOsArm64:
      remoteFileName = `cargo-generate-${tag}-aarch64-apple-darwin.tar.gz`;
      break;
  }

  await downloadBinIfNotExists({
    localFilePath,
    dirPath,
    owner: "cargo-generate",
    repoName: "cargo-generate",
    remoteFileName,
  });
}
