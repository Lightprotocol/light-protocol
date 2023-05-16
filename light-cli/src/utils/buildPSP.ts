import { createVerifyingkeyRsFile } from "./createRustVerifyingKey";
import * as fs from "fs";
import * as path from "path";
import { execSync } from "child_process";
import { randomBytes } from "tweetnacl";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { utils } from "@coral-xyz/anchor";
import { sleep } from "@lightprotocol/zk.js";
import { downloadFileIfNotExists } from "./downloadBin";

/**
 * Generates a zk-SNARK circuit given a circuit name.
 * Downloads the required powers of tau file if not available.
 * Compiles the circuit, performs the groth16 setup, and exports the verification key.
 * Cleans up temporary files upon completion.
 * @param circuitName - The name of the circuit to be generated.
 * @returns {Promise<void>}
 */
async function generateCircuit(
  circuitName: string,
  ptau: number,
  programName: string
): Promise<void> {
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
    execSync(
      `curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_${POWERS_OF_TAU}.ptau --create-dirs -o ${ptauFilePath}`
    );
  }

  const stdout = execSync(
    `circom --r1cs --wasm --sym ./circuit/${circuitName}.circom -o ${sdkBuildCircuitDir}/`
  );
  console.log(stdout.toString().trim());

  const stdoutSetup = execSync(
    `yarn snarkjs groth16 setup ${sdkBuildCircuitDir}/${circuitName}.r1cs ${ptauFilePath} ${sdkBuildCircuitDir}/${circuitName}_tmp.zkey`
  );
  console.log("groth16 test setup \n", stdoutSetup.toString().trim());

  let randomContributionBytes = utils.bytes.hex.encode(
    Buffer.from(randomBytes(128))
  );
  try {
    fs.unlinkSync(`${sdkBuildCircuitDir}/${circuitName}.zkey`);
  } catch (_) {}
  const stdoutContribution = execSync(
    `yarn snarkjs zkey contribute ${sdkBuildCircuitDir}/${circuitName}_tmp.zkey ${sdkBuildCircuitDir}/${circuitName}.zkey -e="${randomContributionBytes}"`
  );
  console.log(
    "groth16 test setup contribution \n",
    stdoutContribution.toString().trim()
  );

  execSync(
    `yarn snarkjs zkey export verificationkey ${sdkBuildCircuitDir}/${circuitName}.zkey ${sdkBuildCircuitDir}/verifyingkey.json`
  );
  const vKeyJsonPath = "./build-circuit/verifyingkey.json";
  const vKeyRsPath = "./programs/" + programName + "/src/verifying_key.rs";
  const artifiactPath = "./build-circuit/" + circuitName;
  try {
    fs.unlinkSync(vKeyJsonPath);
  } catch (_) {}
  while (!fs.existsSync(vKeyJsonPath)) {
    execSync(
      `yarn snarkjs zkey export verificationkey ${sdkBuildCircuitDir}/${circuitName}.zkey ${sdkBuildCircuitDir}/verifyingkey.json`
    );
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
    artifiactPath
  );
  console.log("created rust verifying key");
  const sleep = (ms: number) => {
    return new Promise((resolve) => setTimeout(resolve, ms));
  };
  while (!fs.existsSync(vKeyRsPath)) {
    await sleep(10);
  }

  fs.unlinkSync(path.join(sdkBuildCircuitDir, "verifyingkey.json"));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}_tmp.zkey`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}.r1cs`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}.sym`));
}

/**
 * Searches for a file with the ".light" extension in a specified directory.
 * Throws an error if more than one such file or no such file is found.
 * @param directory - The directory to search for the .light file.
 * @returns {string} - The name of the .light file found in the directory.
 */
function findLightFile(directory: string): string {
  const files = fs.readdirSync(directory);
  const lightFiles = files.filter((file) => file.endsWith(".light"));

  if (lightFiles.length > 1) {
    throw new Error("More than one .light file found in the directory.");
  } else if (lightFiles.length === 1) {
    return lightFiles[0];
  } else {
    throw new Error("No .light files found in the directory.");
  }
}

/**
 * Extracts the circuit filename from the input string using a regex pattern.
 * @param input - The string to extract the circuit filename from.
 * @returns {string | null} - The extracted circuit filename or null if not found.
 */
function extractFilename(input: string): string | null {
  const regex = /main\s+(\S+\.circom)/;
  const match = input.match(regex);

  return match ? match[1] : null;
}

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
  let circuitFileName = findLightFile(circuitDir);
  console.log("Creating circom files");
  const macroCircomBinPath = path.resolve(__dirname, "../../bin/macro-circom");
  // TODO: check whether circom binary exists if not load it
  const dirPath = path.resolve(__dirname, "../../bin/");

  await downloadFileIfNotExists({
    filePath: macroCircomBinPath,
    dirPath,
    repoName: "macro-circom",
    fileName: "macro-circom",
  });

  let stdout = execSync(
    `${macroCircomBinPath} ./${circuitDir}/${circuitFileName} ${programName}`
  );
  
  console.log(stdout.toString().trim());

  const circuitMainFileName = extractFilename(stdout.toString().trim());
  console.log("Building circom circuit ", circuitMainFileName);
  if (!circuitMainFileName)
    throw new Error("Could not extract circuit main file name");

  const suffix = ".circom";
  console.log("generateCircuit");

  await generateCircuit(
    circuitMainFileName.slice(0, -suffix.length),
    ptau,
    programName
  );
  const lightAnchorBinPath = path.resolve(__dirname, "../../bin/light-anchor");
  console.log("\nbuilding anchor program\n");
  execSync(`${lightAnchorBinPath} build`);
  console.log("anchor build success");
}

export function toSnakeCase(str: string): string {
  return str.replace(/-/g, "_");
}
