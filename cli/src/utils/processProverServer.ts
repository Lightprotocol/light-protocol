import path from "path";
import { killProcessByName, killProcessByPort, spawnBinary } from "./process";
import { LIGHT_PROVER_PROCESS_NAME } from "./constants";
import { sleep } from "@lightprotocol/stateless.js";
import { killProcess, waitForServers } from "./initTestEnv";

const KEYS_DIR = "proving-keys/";

export async function killProver() {
  await killProcess(getProverNameByArch());
  // Temporary fix for the case when prover is instantiated via prover.sh:
  await killProcess("light-prover");
}

export async function startProver(
  proveCompressedAccounts: boolean,
  proveNewAddresses: boolean,
) {
  if (!proveCompressedAccounts && !proveNewAddresses) {
    console.log(
      "No flags provided. Please provide at least one flag to start the prover.",
    );
    process.exit(1);
  }
  console.log("Kill existing prover process...");
  await killProcessByName(LIGHT_PROVER_PROCESS_NAME);
  await killProcessByName(getProverNameByArch());
  await killProcessByPort("3001");

  const keysDir = path.join(__dirname, "../..", "bin", KEYS_DIR);
  const args = ["start"];
  args.push(`--inclusion=${proveCompressedAccounts ? "true" : "false"}`);
  args.push(`--non-inclusion=${proveNewAddresses ? "true" : "false"}`);
  args.push("--keys-dir", keysDir);

  console.log("Starting prover...");
  spawnBinary(getProverNameByArch(), true, args);
  await waitForServers([{ port: 3001, path: "/" }]);
  console.log("Prover started successfully!");
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

export type ProofInputs = {
  roots: string[];
  inPathIndices: number[];
  inPathElements: string[][];
  leaves: string[];
};
export function provingArgs(inputs: string): string {
  const arg0 = "echo";
  const arg1 = inputs;
  const arg2 = `bin/${getProverNameByArch()}`;
  const arg3 = "prove";

  const arg4 = provingKey(parseProofInputs(inputs).roots.length);
  const args = [
    arg0,
    "'",
    arg1,
    "' | ",
    arg2,
    arg3,
    arg4,
    "--inclusion",
    "--keys-dir",
    `./bin/${KEYS_DIR}/`,
  ].join(" ");
  return args;
}

export function verifyingArgs(
  proof: string,
  roots: string[],
  leafs: string[],
): string {
  const arg0 = "echo";
  const arg1 = proof;
  const arg2 = "./bin/light-prover";
  const arg3 = "verify";
  const arg4 = provingKey(roots.length);
  const arg5 = `--roots ${roots}`;
  const arg6 = `--leafs ${leafs}`;

  const args = [arg0, "'", arg1, "' | ", arg2, arg3, arg4, arg5, arg6].join(
    " ",
  );

  return args;
}

function provingKey(utxos: number, height: number = 26): string {
  return `-k ./bin/${KEYS_DIR}inclusion_${height}_${utxos}.key`;
}

function parseProofInputs(json: string): ProofInputs {
  const inputs: ProofInputs = JSON.parse(json);
  return inputs;
}
