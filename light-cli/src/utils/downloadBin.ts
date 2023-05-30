import axios from "axios";
import * as fs from "fs";
import { promisify } from "util";
import * as os from "os";

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

function getSystem(): string {
  const arch = os.arch();
  const platform = os.platform();

  let platformName: string;
  let archName: string;

  if (platform === "darwin") {
    platformName = "macos";
  } else if (platform === "linux") {
    platformName = "linux";
  } else {
    throw new Error(`Platform ${platform} is not supported.`);
  }

  if (arch === "x64") {
    archName = "amd64";
  } else if (arch === "arm" || arch === "arm64") {
    archName = "arm64";
  } else {
    throw new Error(`Architecture ${arch} is not supported.`);
  }

  return `${platformName}-${archName}`;
}

function makeExecutable(filePath: string): void {
  fs.chmodSync(filePath, "755");
}

export async function downloadFileIfNotExists({
  filePath,
  dirPath,
  repoName,
  fileName,
}: {
  filePath: string;
  dirPath: string;
  repoName: string;
  fileName: string;
}) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }

  // Check if file exists
  if (await fileExists(filePath)) {
    return;
  }

  const system = getSystem();
  const tag = await latestRelease("lightprotocol", repoName);

  const url = `https://github.com/Lightprotocol/${repoName}/releases/download/${tag}/${fileName}-${system}`;

  if (!url) {
    throw new Error(`No binary found for the detected system ${system}`);
  }

  // Download the file
  console.log(`Downloading ${fileName} from ${url}...`);
  const { data } = await axios({
    url,
    method: "GET",
    responseType: "stream",
  });

  // Save the file
  const writer = fs.createWriteStream(filePath);
  data.pipe(writer);

  return new Promise<void>((resolve, reject) => {
    writer.on("finish", () => {
      makeExecutable(filePath); // Make the file executable after it has been written.
      resolve();
    });
    writer.on("error", reject);
  });
}