import { executeAnchor, executeMacroCircom } from "./toolchain";
import { extractFilename, findFile } from "./utils";
import { generateCircuit } from "./buildCircom";
/**
 * Builds a Private Solana Program (PSP) given a circuit directory.
 * Creates circom files, builds the circom circuit, and compiles the anchor program.
 * @param circuitDir - The directory containing the circuit files.
 * @returns {Promise<void>}
 */
export async function buildPSP(
  circuitDir: string,
  ptau: number,
  programName: string
) {
  let circuitFileName = findFile({
    directory: circuitDir,
    extension: "light",
  });

  console.log("📜 Generating circom files");
  let stdout = await executeMacroCircom({
    args: [`./${circuitDir}/${circuitFileName}`, programName],
  });
  console.log("✅ Circom files generated successfully");

  const circuitMainFileName = extractFilename({
    file: stdout.toString().trim(),
    suffix: "circom",
  });

  console.log("🛠️️  Building circuit", circuitMainFileName);
  if (!circuitMainFileName)
    throw new Error("Could not extract circuit main file name");

  console.log("🔑 Generating circuit");
  await generateCircuit({
    circuitFileName: circuitMainFileName,
    ptau,
    programName,
  });
  console.log("✅ Circuit generated successfully");

  console.log("🛠  Building on-chain program");
  await executeAnchor({ args: ["build"] });
  console.log("✅ Build finished successfully");
}
