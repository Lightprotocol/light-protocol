import axios from 'axios';
import * as fs from 'fs';
import { promisify } from 'util';
import * as os from 'os';

const fileExists = promisify(fs.exists);

export const anchorBinUrlMap = new Map([
  ["linux-amd64", "https://github.com/Lightprotocol/anchor/releases/download/v0.27.0/light-anchor-linux-amd64"],
  ["macos-amd64", "https://github.com/Lightprotocol/anchor/releases/download/v0.27.0/light-anchor-macos-amd64"],
  ["macos-arm64", "https://github.com/Lightprotocol/anchor/releases/download/v0.27.0/light-anchor-macos-arm64"],
  ["linux-arm64", "https://github.com/Lightprotocol/anchor/releases/download/v0.27.0/light-anchor-linux-arm64"]
])

export const macroCircomBinUrlMap = new Map([
  ["linux-amd64", "https://github.com/Lightprotocol/macro-circom/releases/download/v0.1.1/macro-circom-linux-amd64"],
  ["macos-amd64", "https://github.com/Lightprotocol/macro-circom/releases/download/v0.1.1/macro-circom-macos-amd64"],
  ["macos-arm64", "https://github.com/Lightprotocol/macro-circom/releases/download/v0.1.1/macro-circom-linux-arm64"],
  ["linux-arm64", "https://github.com/Lightprotocol/macro-circom/releases/download/v0.1.1/macro-circom-macos-arm64"]
])

function getSystem(): string {
  const arch = os.arch();
  const platform = os.platform();

  let platformName: string;
  let archName: string;

  if (platform === 'darwin') {
    platformName = 'macos';
  } else if (platform === 'linux') {
    platformName = 'linux';
  } else {
    throw new Error(`Platform ${platform} is not supported.`);
  }

  if (arch === 'x64') {
    archName = 'amd64';
  } else if (arch === 'arm' || arch === 'arm64') {
    archName = 'arm64';
  } else {
    throw new Error(`Architecture ${arch} is not supported.`);
  }

  return `${platformName}-${archName}`;
}

function makeExecutable(filePath: string): void {
  fs.chmodSync(filePath, '755');
}

export async function downloadFileIfNotExists(urlMap: Map<string, string>, filePath: string, dirPath: string, name: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }

  // Check if file exists
  if (await fileExists(filePath)) {
    return;
  }

  const system = getSystem()
  const url = urlMap.get(system);

  if (!url) {
    throw new Error(`No binary found for the detected system ${system}`);
  }

  // Download the file
  console.log(` ${name} binary does not exist, starting download...`);
  const { data } = await axios({
    url,
    method: 'GET',
    responseType: 'stream',
  });

  // Save the file
  const writer = fs.createWriteStream(filePath);
  data.pipe(writer);

  return new Promise<void>((resolve, reject) => {
    writer.on('finish', () => {
      makeExecutable(filePath); // Make the file executable after it has been written.
      resolve();
    });
    writer.on('error', reject);
  });
}
