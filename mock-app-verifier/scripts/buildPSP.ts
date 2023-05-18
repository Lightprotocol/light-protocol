import { buildPSP } from "../../cli/src/utils/buildPSP";

async function main() {
  let circuitDir = process.argv[2];
  let programName = process.argv[3];
  if (!circuitDir) {
    throw new Error("circuitDir is not specified as argument!");
  }
  await buildPSP(circuitDir, 14, programName);
}

main();
