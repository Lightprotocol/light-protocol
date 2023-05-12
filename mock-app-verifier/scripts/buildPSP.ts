import { createVerfyingkeyRsFile } from "../../light-sdk-ts/src/cli-utils/createRustVerifyingKey";
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

/**
* Generates a zk-SNARK circuit given a circuit name.
* Downloads the required powers of tau file if not available.
* Compiles the circuit, performs the groth16 setup, and exports the verification key.
* Cleans up temporary files upon completion.
* @param circuitName - The name of the circuit to be generated.
* @returns {Promise<void>}
*/
async function generateCircuit(circuitName: string): Promise<void> {

  const POWERS_OF_TAU = 17;
  const ptauFileName = `ptau${POWERS_OF_TAU}`;
  const buildDir = path.join('build');
  const sdkBuildCircuitDir = path.join('sdk', 'build-circuit');

  if (!fs.existsSync(buildDir)) {
    fs.mkdirSync(buildDir, { recursive: true });
  }

  const ptauFilePath = path.join(buildDir, ptauFileName);
  if (!fs.existsSync(ptauFilePath)) {
    console.log('Downloading powers of tau file');
    execSync(`curl -L https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_${POWERS_OF_TAU}.ptau --create-dirs -o ${ptauFilePath}`);
  }

  const stdout = execSync(`circom --r1cs --wasm --sym ./circuit/${circuitName}.circom -o ${sdkBuildCircuitDir}/`);

  const output = stdout.toString().trim();
  console.log(output);
  
  const stdoutSetup =execSync(`yarn snarkjs groth16 setup ${sdkBuildCircuitDir}/${circuitName}.r1cs ${ptauFilePath} ${sdkBuildCircuitDir}/${circuitName}.zkey`);

  console.log("groth16 test setup complete \n", stdoutSetup.toString().trim());
  execSync(`yarn snarkjs zkey export verificationkey ${sdkBuildCircuitDir}/${circuitName}.zkey ${sdkBuildCircuitDir}/verifyingkey.json`);
    const program = "verifier" //`${process.argv[3]}`;
    const vKeyJsonPath = "./sdk/build-circuit/verifyingkey.json";
    const vKeyRsPath = "./programs/" + program + "/src/verifying_key.rs";
    const artifiactPath = "./sdk/build-circuit/" + circuitName;
    while (!fs.existsSync(vKeyJsonPath)) {
        execSync(`yarn snarkjs zkey export verificationkey ${sdkBuildCircuitDir}/${circuitName}.zkey ${sdkBuildCircuitDir}/verifyingkey.json`);
    }
    await createVerfyingkeyRsFile(program, [], vKeyJsonPath, vKeyRsPath, circuitName, artifiactPath);
    console.log("created rust verifying key");

  fs.unlinkSync(path.join(sdkBuildCircuitDir, 'verifyingkey.json'));
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
  const lightFiles = files.filter(file => file.endsWith('.light'));

  if (lightFiles.length > 1) {
    throw new Error('More than one .light file found in the directory.');
  } else if (lightFiles.length === 1) {
    return lightFiles[0];
  } else {
    throw new Error('No .light files found in the directory.');
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
async function buildPSP(circuitDir: string) {
  let circuitFileName = findLightFile(circuitDir)

  console.log("Creating circom files");
  let stdout = execSync(`./../macro-circom/target/debug/rust-circom-dsl ./circuit/${circuitFileName}`);
  console.log(stdout.toString().trim());

  const circuitMainFileName = extractFilename(stdout.toString().trim());
  console.log("Building circom circuit ", circuitMainFileName)

  const suffix = '.circom';
  await generateCircuit(circuitMainFileName.slice(0, -suffix.length));

  console.log("\nbuilding anchor program\n");
  execSync("anchor build");
  console.log("anchor build success");
}

async function main() {
  let circuitDir = process.argv[2];
  if (!circuitDir) {
      throw new Error("circuitDir is not specified as argument!");
  }
  await buildPSP(circuitDir);
}

main()