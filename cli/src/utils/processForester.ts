import path from "path";
import { killProcess, spawnBinary } from "./process";
import { FORESTER_PROCESS_NAME } from "./constants";

export async function killForester() {
  await killProcess(FORESTER_PROCESS_NAME);
}

export async function startForester() {
  console.log("Killing existing forester process...");
  await killForester();

  console.log("Starting forester...");
  spawnBinary(getForesterBinaryName(), ["subscribe"]);
  console.log("Forester started successfully!");
}

export function getForesterBinaryName(): string {
  const binDir = path.join(__dirname, "../..", "bin");
  const binaryName = path.join(binDir, "forester");
  return binaryName;
}
