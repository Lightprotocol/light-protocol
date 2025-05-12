import path from "path";
import {
  executeCommand,
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME } from "./constants";
import { featureFlags } from "@lightprotocol/stateless.js";

const KEYS_DIR = "proving-keys/";

export async function killProver() {
  await killProcess(getProverNameByArch());
  await killProcess(LIGHT_PROVER_PROCESS_NAME);
}

export async function startProver(
  proverPort: number,
  runMode: string | undefined,
  circuits: string[] | undefined = [],
) {
  console.log("Kill existing prover process...");
  await killProver();
  await killProcessByPort(proverPort);

  const keysDir = path.join(__dirname, "../..", "bin", KEYS_DIR);
  const args = ["start"];
  args.push("--keys-dir", keysDir);
  args.push("--prover-address", `0.0.0.0:${proverPort}`);

  if (runMode != null) {
    args.push("--run-mode", runMode);
  }

  // If v1 is used, load only the circuits that are needed for v1. If v2 is
  // used, load all circuits.
  const mappedCircuits = circuits.map((circuit) => {
    if (circuit.includes("append")) {
      // For append circuits, we need to specify both tree height and batch size
      return `append-with-proofs_32_10`; // Using 32 for tree height and 10 for batch size since that's the only key file that exists
    }
    if (circuit === "update" || circuit === "address-append") {
      return circuit;
    }

    const compressedAccounts = [1, 2, 3, 4, 8];
    const v1Circuits = {
      inclusion: compressedAccounts.map((count) => `inclusion_26_${count}`),
      "non-inclusion": ["non-inclusion_26_1", "non-inclusion_26_2"],
      combined: [
        "combined_26_1_1",
        "combined_26_1_2",
        "combined_26_2_1",
        "combined_26_2_2",
        "combined_26_3_1",
        "combined_26_3_2",
        "combined_26_4_1",
        "combined_26_4_2",
      ],
    };

    const v2Circuits = {
      inclusion: compressedAccounts.map((count) => `inclusion_32_${count}`),
      "non-inclusion": [
        "non-inclusion_40_1",
        "non-inclusion_40_2",
        "non-inclusion_40_3",
        "non-inclusion_40_4",
        "non-inclusion_40_8",
      ],
      combined: [
        "combined_32_40_1_1",
        "combined_32_40_1_2",
        "combined_32_40_1_3",
        "combined_32_40_1_4",
        "combined_32_40_2_1",
        "combined_32_40_2_2",
        "combined_32_40_2_3",
        "combined_32_40_2_4",
        "combined_32_40_3_1",
        "combined_32_40_3_2",
        "combined_32_40_3_3",
        "combined_32_40_3_4",
        "combined_32_40_4_1",
        "combined_32_40_4_2",
        "combined_32_40_4_3",
        "combined_32_40_4_4",
      ],
    };

    if (featureFlags.version === "V1") {
      return v1Circuits[circuit as keyof typeof v1Circuits] || circuit;
    } else if (featureFlags.version === "V2") {
      return [
        ...(v1Circuits[circuit as keyof typeof v1Circuits] || []),
        ...(v2Circuits[circuit as keyof typeof v2Circuits] || []),
      ];
    }

    return circuit;
  });

  const flattenedCircuits = mappedCircuits.flat();

  for (const circuit of flattenedCircuits) {
    args.push("--circuit", circuit);
  }

  if (runMode != null) {
    console.log(`Starting prover in ${runMode} mode...`);
  } else if (circuits && circuits.length > 0) {
    console.log(`Starting prover with ${featureFlags.version} circuits...`);
    console.log(`Circuit variants loaded: ${flattenedCircuits.length}`);
  }

  if ((!circuits || circuits.length === 0) && runMode == null) {
    runMode = "rpc";
    args.push("--run-mode", runMode);
    console.log(`Starting prover with fallback ${runMode} mode...`);
  }

  console.log(`Starting prover with ${featureFlags.version} configuration...`);

  spawnBinary(getProverPathByArch(), args);
  await waitForServers([{ port: proverPort, path: "/" }]);
  console.log(`Prover started successfully!`);
}

export function getProverNameByArch(): string {
  const platform = process.platform;
  const arch = process.arch;

  if (!platform || !arch) {
    throw new Error("Unsupported platform or architecture");
  }

  let binaryName = `prover-${platform}-${arch}`;

  if (platform.toString() === "windows") {
    binaryName += ".exe";
  }
  return binaryName;
}

export function getProverPathByArch(): string {
  let binaryName = getProverNameByArch();
  // We need to provide the full path to the binary because it's not in the PATH.
  const binDir = path.join(__dirname, "../..", "bin");
  binaryName = path.join(binDir, binaryName);

  return binaryName;
}
