import { executeAnchor, executeMacroCircom } from "./toolchain";
import { extractFilename, findFile } from "./utils";
import { generateCircuit } from "./buildCircom";

/**
 * Generates a zk-SNARK circuit given a circuit name.
 * Downloads the required powers of tau file if not available.
 * Compiles the circuit, performs the groth16 setup, and exports the verification key.
 * Cleans up temporary files upon completion.
 * @param circuitName - The name of the circuit to be generated.
 * @returns {Promise<void>}
 
async function generateCircuit({
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
  const vKeyJsonPath = `./build-circuit/verifyingkey${circuitName}.json`;
  const vKeyRsPath = "./programs/" + programName + `/src/verifying_key_${toSnakeCase(circuitName)}.rs`;
  const artifactPath = "./build-circuit/" + circuitName;
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

  fs.unlinkSync(path.join(sdkBuildCircuitDir, `verifyingkey${circuitName}.json`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}_tmp.zkey`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}Main.r1cs`));
  fs.unlinkSync(path.join(sdkBuildCircuitDir, `${circuitName}Main.sym`));
}
*/

/**
 * Builds a Private Solana Program (PSP) given a circuit directory.
 * Creates circom files, builds the circom circuit, and compiles the anchor program.
 * @param circuitDir - The directory containing the circuit files.
 * @returns {Promise<void>}
 */
export async function buildPSP({
  circuitDir,
  ptau,
  programName,
  onlyCircuit,
  skipCircuit,
  skipMacroCircom,
  addCircuitName,
}: {
  circuitDir: string;
  ptau: number;
  programName: string;
  onlyCircuit?: boolean;
  skipCircuit?: boolean;
  skipMacroCircom?: boolean;
  addCircuitName?: string[];
}) {
  if (!skipCircuit) {
    if (!skipMacroCircom) {
      let circuitFileName = findFile({
        directory: circuitDir,
        extension: "light",
      });

      console.log("📜 Generating circom files");
      let stdout = await executeMacroCircom({
        args: [`./${circuitDir}/${circuitFileName}`, programName],
      });
      console.log("✅ Circom files generated successfully");
      const circuitMainFileName = extractFilename(stdout.toString().trim());
      console.log("🛠️️  Building circuit", circuitMainFileName);
      if (!circuitMainFileName)
        throw new Error("Could not extract circuit main file name");

      const suffix = ".circom";

      console.log("🔑 Generating circuit");
      await generateCircuit({
        circuitName: circuitMainFileName.slice(0, -suffix.length),
        ptau,
        programName,
      });
      console.log("✅ Circuit generated successfully");
    }

    // TODO: enable multiple programs
    // TODO: add add-psp command which adds a second psp
    // TODO: add add-circom-circuit command which inits a new circom circuit of name circuitName
    // TODO: add add-circuit command which inits a new .light file of name circuitName
    if (addCircuitName) {
      for (let circuitName of addCircuitName) {
        console.log("🔑 Generating circuit ", addCircuitName);
        // TODO: add circuit name to verifying key.rs file
        await generateCircuit({
          circuitName,
          ptau,
          programName,
        });
      }
    }

    if (onlyCircuit) return;
  }
  console.log("🛠  Building on-chain program");
  await executeAnchor({ args: ["build"] });
  console.log("✅ Build finished successfully");
}
