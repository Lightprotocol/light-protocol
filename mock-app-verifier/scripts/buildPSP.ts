import { buildPSP } from "light-sdk";

async function main() {
  let circuitDir = process.argv[2];
  if (!circuitDir) {
      throw new Error("circuitDir is not specified as argument!");
  }
  await buildPSP(circuitDir, 14, "verifier");
}

main()