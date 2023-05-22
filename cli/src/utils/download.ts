import axios from "axios";
import * as fs from "fs";
import { promisify } from "util";
import cliProgress from "cli-progress";
import * as os from "os";
import * as path from "path";
import * as tar from "tar";
import * as zlib from "zlib";

const fileExists = promisify(fs.exists);

async function latestRelease(owner: string, repo: string) {
  const github = "https://api.github.com";
  console.log(
    `Checking the latest release of ${github}/repos/${owner}/${repo}/releases/latest`
  );

  const response = await axios.get(
    `${github}/repos/${owner}/${repo}/releases/latest`
  );
  const tag_name = response.data.tag_name;

  console.log(`The newest release of ${repo} is ${tag_name}`);

  return response.data.tag_name;
}

enum System {
  MacOsAmd64,
  MacOsArm64,
  LinuxAmd64,
  LinuxArm64,
}

function getSystem(): System {
  const arch = os.arch();
  const platform = os.platform();

  switch (platform) {
    case "darwin":
      switch (arch) {
        case "x64":
          return System.MacOsAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.MacOsArm64;
        default:
          throw new Error(`Architecture ${arch} is not supported.`);
      }
    case "linux":
      switch (arch) {
        case "x64":
          return System.LinuxAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.LinuxArm64;
        default:
          throw new Error(`Architecture ${arch} is not supported.`);
      }
  }

  throw new Error(`Platform ${platform} is not supported.`);
}

function makeExecutable(filePath: string): void {
  fs.chmodSync(filePath, "755");
}

/**
 * Downloads a file from the given URL to the given local file path.
 * @param localFilePath - The local file path to download the file to.
 * @param url - The URL to download the file from.
 * @returns {Promise<void>}
 */
export async function downloadFile({
  localFilePath,
  url,
}: {
  localFilePath: string;
  url: string;
}) {
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

  // If the file is a tar.gz file, unzip and untar it while it's being written.
  if (url.endsWith(".tar.gz")) {
    console.log(`Extracting ${url}...`);
    const gunzip = zlib.createGunzip();
    const parser = new tar.Parse();
    data.pipe(gunzip).pipe(parser);

    // Sadly, `tar` doesn't expose any interface which would describe all
    // properties we need, so we have to use `any` here.
    parser.on("entry", (entry: any) => {
      if (entry.path === path.parse(localFilePath).base) {
        entry.pipe(fs.createWriteStream(localFilePath));
      } else {
        entry.resume();
      }
    });

    return new Promise<void>((resolve, reject) => {
      parser.on("end", () => {
        // Make the file executable after it has been written.
        makeExecutable(localFilePath);
        resolve();
      });
    });
  } else {
    let writeStream = fs.createWriteStream(localFilePath);
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
 * @param dirPath - The path to the directory where the file will be created.
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
}: {
  localFilePath: string;
  dirPath: string;
  owner: string;
  repoName: string;
  remoteFileName: string;
}) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }

  // Check if file exists
  if (await fileExists(localFilePath)) {
    return;
  }

  const tag = await latestRelease(owner, repoName);
  const url = `https://github.com/${owner}/${repoName}/releases/download/${tag}/${remoteFileName}`;

  // Download the file
  console.log(`Downloading ${remoteFileName} from ${url}...`);

  await downloadFile({
    localFilePath,
    url,
  });
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
}: {
  localFilePath: string;
  dirPath: string;
  repoName: string;
  remoteFileName: string;
}) {
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

  const fullRemoteFileName = `${remoteFileName}-${systemSuffix}`;
  await downloadBinIfNotExists({
    localFilePath,
    dirPath,
    owner: "Lightprotocol",
    repoName,
    remoteFileName: fullRemoteFileName,
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
      remoteFileName =
        `cargo-generate-${tag}-aarch64-unknown-linux-musl.tar.gz`;
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
