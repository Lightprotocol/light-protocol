import { createVerifyingkeyRsFile } from "./index";
import * as fs from "fs";
import * as path from "path";
import { randomBytes } from "tweetnacl";
import { utils } from "@coral-xyz/anchor";
import { downloadFile } from "./download";
import { executeCommand } from "./process";
import { executeAnchor, executeCircom } from "./toolchain";
import { findFile } from "./utils";
import { toSnakeCase } from "@lightprotocol/zk.js";

/**
 * Generates a zk-SNARK circuit given a circuit name.
 * Downloads the required powers of tau file if not available.
 * Compiles the circuit, performs the groth16 setup, and exports the verification key.
 * Cleans up temporary files upon completion.
 * @param circuitName - The name of the circuit to be generated.
 * @returns {Promise<void>}
 */
export async function generateCircuit({
  circuitName,
  ptau,
  programName,
  circuitPath = "./circuits",
}: {
  circuitName: string;
  ptau: number;
  programName: string;
  circuitPath?: string;
}): Promise<void> {
  // const circuitName = circuitFileName.slice(0, -".circom".length);
  const POWERS_OF_TAU = ptau;
  const ptauFileName = `ptau${POWERS_OF_TAU}`;
  const buildDir = "./target";
  const sdkBuildCircuitDir = "./build-circuit";

  if (!fs.existsSync(buildDir)) {
    fs.mkdirSync(buildDir, { recursive: true });
  }

  if (!fs.existsSync(sdkBuildCircuitDir)) {
    fs.mkdirSync(sdkBuildCircuitDir, { recursive: true });
  }

  const ptauFilePath = buildDir + "/" + ptauFileName;
  if (!fs.existsSync(ptauFilePath)) {
    console.log("Downloading powers of tau file");
    await downloadFile({
      localFilePath: ptauFilePath,
      dirPath: buildDir,
      url: `https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_${POWERS_OF_TAU}.ptau`,
    });
  }

  await executeCircom({
    args: [
      "--r1cs",
      "--wasm",
      "--sym",
      `${circuitPath}/${circuitName}/${circuitName}Main.circom`,
      "-o",
      `${sdkBuildCircuitDir}/`,
    ],
  });

  await executeCommand({
    command: "yarn",
    args: [
      "snarkjs",
      "groth16",
      "setup",
      `${sdkBuildCircuitDir}/${circuitName}Main.r1cs`,
      ptauFilePath,
      `${sdkBuildCircuitDir}/${circuitName}_tmp.zkey`,
    ],
  });

  let randomContributionBytes = utils.bytes.hex.encode(
    Buffer.from(randomBytes(128))
  );
  try {
    fs.unlinkSync(`${sdkBuildCircuitDir}/${circuitName}.zkey`);
  } catch (_) {}
  await executeCommand({
    command: "yarn",
    args: [
      "snarkjs",
      "zkey",
      "contribute",
      `${sdkBuildCircuitDir}/${circuitName}_tmp.zkey`,
      `${sdkBuildCircuitDir}/${circuitName}.zkey`,
      `-e=${randomContributionBytes}`,
    ],
  });

  await executeCommand({
    command: "yarn",
    args: [
      "snarkjs",
      "zkey",
      "export",
      "verificationkey",
      `${sdkBuildCircuitDir}/${circuitName}.zkey`,
      `${sdkBuildCircuitDir}/verifyingkey${circuitName}.json`,
    ],
  });

  const vKeyJsonPath = sdkBuildCircuitDir + `/verifyingkey${circuitName}.json`;
  const vKeyRsPath =
    "./programs/" +
    programName +
    `/src/verifying_key_${toSnakeCase(circuitName)}.rs`;
  const artifactPath = sdkBuildCircuitDir + "/" + circuitName;
  try {
    fs.unlinkSync(vKeyJsonPath);
  } catch (_) {}
  while (!fs.existsSync(vKeyJsonPath)) {
    await executeCommand({
      command: "yarn",
      args: [
        "snarkjs",
        "zkey",
        "export",
        "verificationkey",
        `${sdkBuildCircuitDir}/${circuitName}.zkey`,
        `${sdkBuildCircuitDir}/verifyingkey${circuitName}.json`,
      ],
    });
  }
  try {
    fs.unlinkSync(vKeyRsPath);
  } catch (_) {}
  await createVerifyingkeyRsFile(
    programName,
    [],
    vKeyJsonPath,
    vKeyRsPath,
    circuitName,
    artifactPath
  );
  console.log("created rust verifying key");
  const sleep = (ms: number) => {
    return new Promise((resolve) => setTimeout(resolve, ms));
  };
  while (!fs.existsSync(vKeyRsPath)) {
    await sleep(10);
  }

  fs.unlinkSync(
    path.join(sdkBuildCircuitDir, `verifyingkey${circuitName}.json`)
  );
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}_tmp.zkey`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}Main.r1cs`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}Main.sym`));
}

/**
 * Builds a barebone Circom + Anchor project given a circuit directory.
 * Initializes client-side typescript prover, on-chain groth16-solana verifier, builds the circom circuit, and compiles the anchor program.
 * @param circuitDir - The directory containing the circuit files.
 * @returns {Promise<void>}
 */
export async function buildCircom(
  circuitDir: string,
  ptau: number,
  programName: string
) {
  let circuitFileName = findFile({
    directory: circuitDir,
    extension: "circom",
  });

  console.log("🛠️️  Building circuit", circuitFileName);
  const suffix = ".circom";

  await generateCircuit({
    circuitName: circuitFileName.slice(0, -suffix.length),
    ptau,
    programName,
  });
  console.log("✅ Circuit generated successfully");

  console.log("🛠  Building on-chain program");
  await executeAnchor({ args: ["build"] });
  console.log("✅ Build finished successfully");
}
