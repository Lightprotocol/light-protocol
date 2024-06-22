import { airdropSol } from "@lightprotocol/stateless.js";
import { getConfig, getPayer, setAnchorProvider, setConfig } from "./utils";
import {
  BASE_PATH,
  LIGHT_ACCOUNT_COMPRESSION_TAG,
  LIGHT_COMPRESSED_TOKEN_TAG,
  LIGHT_PROTOCOL_PROGRAMS_DIR_ENV,
  LIGHT_REGISTRY_TAG,
  LIGHT_SYSTEM_PROGRAM_TAG,
  SPL_NOOP_PROGRAM_TAG,
} from "./constants";
import path from "path";
import { downloadBinIfNotExists } from "../psp-utils";
import {
  confirmServerStability,
  executeCommand,
  killProcess,
  waitForServers,
} from "./process";
import { startProver } from "./processProverServer";
import { startIndexer } from "./processPhotonIndexer";
import { startForester } from "./processForester";

export async function initTestEnv({
  additionalPrograms,
  skipSystemAccounts,
  indexer = true,
  prover = true,
  forester = true,
  rpcPort = 8899,
  indexerPort = 8784,
  proverPort = 3001,
  gossipHost = "127.0.0.1",
  proveCompressedAccounts = true,
  proveNewAddresses = false,
  checkPhotonVersion = true,
  photonDatabaseUrl,
  limitLedgerSize,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  indexer: boolean;
  prover: boolean;
  forester: boolean;
  rpcPort?: number;
  indexerPort?: number;
  proverPort?: number;
  gossipHost?: string;
  proveCompressedAccounts?: boolean;
  proveNewAddresses?: boolean;
  checkPhotonVersion?: boolean;
  photonDatabaseUrl?: string;
  limitLedgerSize?: number;
}) {
  console.log("Performing setup tasks...\n");

  const initAccounts = async () => {
    const anchorProvider = await setAnchorProvider();
    const payer = await getPayer();
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 100e9,
      recipientPublicKey: payer.publicKey,
    });
  };
  // We cannot await this promise directly because it will hang the process
  startTestValidator({
    additionalPrograms,
    skipSystemAccounts,
    limitLedgerSize,
    rpcPort,
    gossipHost,
  });
  await waitForServers([{ port: rpcPort, path: "/health" }]);
  await confirmServerStability(`http://127.0.0.1:${rpcPort}/health`);
  await initAccounts();

  if (indexer) {
    const config = getConfig();
    config.indexerUrl = `http://127.0.0.1:${indexerPort}`;
    setConfig(config);
    await startIndexer(
      `http://127.0.0.1:${rpcPort}`,
      indexerPort,
      checkPhotonVersion,
      photonDatabaseUrl,
    );
  }

  if (prover) {
    const config = getConfig();
    config.proverUrl = `http://127.0.0.1:${proverPort}`;
    setConfig(config);
    await startProver(proverPort, proveCompressedAccounts, proveNewAddresses);
  }

  if (forester) {
    await startForester();
  }
}

export async function initTestEnvIfNeeded({
  additionalPrograms,
  skipSystemAccounts,
  indexer = false,
  prover = false,
  forester = false,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  indexer?: boolean;
  prover?: boolean;
  forester?: boolean;
} = {}) {
  try {
    const anchorProvider = await setAnchorProvider();
    // this request will fail if there is no local test validator running
    const payer = await getPayer();
    await anchorProvider.connection.getBalance(payer.publicKey);
  } catch (error) {
    // launch local test validator and initialize test environment
    await initTestEnv({
      additionalPrograms,
      skipSystemAccounts,
      indexer,
      prover,
      forester,
    });
  }
}

/*
 * Determines a path to which Light Protocol programs should be downloaded.
 *
 * If the `LIGHT_PROTOCOL_PROGRAMS_DIR` environment variable is set, the path
 * provided in it is used.
 *
 * Otherwise, the `bin` directory in the CLI internals is used.
 *
 * @returns {string} Directory path for Light Protocol programs.
 */
function programsDirPath(): string {
  return (
    process.env[LIGHT_PROTOCOL_PROGRAMS_DIR_ENV] ||
    path.resolve(__dirname, BASE_PATH)
  );
}

/*
 * Determines a patch to which the given program should be downloaded.
 *
 * If the `LIGHT_PROTOCOL_PROGRAMS_DIR` environment variable is set, the path
 * provided in it is used as a parent
 *
 * Otherwise, the `bin` directory in the CLI internals is used.
 *
 * @returns {string} Path for the given program.
 */
function programFilePath(programName: string): string {
  const programsDir = process.env[LIGHT_PROTOCOL_PROGRAMS_DIR_ENV];
  if (programsDir) {
    return path.join(programsDir, programName);
  }

  return path.resolve(__dirname, path.join(BASE_PATH, programName));
}

export async function getSolanaArgs({
  additionalPrograms,
  skipSystemAccounts,
  limitLedgerSize,
  rpcPort,
  gossipHost,
  downloadBinaries = true,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  limitLedgerSize?: number;
  rpcPort?: number;
  gossipHost?: string;
  downloadBinaries?: boolean;
}): Promise<Array<string>> {
  type Program = { id: string; name?: string; tag?: string; path?: string };
  // TODO: adjust program tags
  const programs: Program[] = [
    {
      id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
      name: "spl_noop.so",
      tag: SPL_NOOP_PROGRAM_TAG,
    },
    {
      id: "H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN",
      name: "light_system_program.so",
      tag: LIGHT_SYSTEM_PROGRAM_TAG,
    },
    {
      id: "HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN",
      name: "light_compressed_token.so",
      tag: LIGHT_COMPRESSED_TOKEN_TAG,
    },
    {
      id: "CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK",
      name: "account_compression.so",
      tag: LIGHT_ACCOUNT_COMPRESSION_TAG,
    },
    {
      id: "7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1",
      name: "light_registry.so",
      tag: LIGHT_REGISTRY_TAG,
    },
  ];
  if (additionalPrograms)
    additionalPrograms.forEach((program) => {
      programs.push({ id: program.address, path: program.path });
    });

  const dirPath = programsDirPath();

  const solanaArgs = [
    "--reset",
    `--limit-ledger-size=${limitLedgerSize}`,
    `--rpc-port=${rpcPort}`,
    `--gossip-host=${gossipHost}`,
    "--quiet",
  ];

  for (const program of programs) {
    if (program.path) {
      solanaArgs.push("--bpf-program", program.id, program.path);
    } else {
      const localFilePath = programFilePath(program.name!);
      if (program.name === "spl_noop.so" || downloadBinaries) {
        await downloadBinIfNotExists({
          localFilePath,
          dirPath,
          owner: "Lightprotocol",
          repoName: "light-protocol",
          remoteFileName: program.name!,
          tag: program.tag,
        });
      }
      solanaArgs.push("--bpf-program", program.id, localFilePath);
    }
  }
  if (!skipSystemAccounts) {
    const accountsRelPath = "../../accounts";
    const accountsPath = path.resolve(__dirname, accountsRelPath);
    solanaArgs.push("--account-dir", accountsPath);
  }

  return solanaArgs;
}

export async function startTestValidator({
  additionalPrograms,
  skipSystemAccounts,
  limitLedgerSize,
  rpcPort,
  gossipHost,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  limitLedgerSize?: number;
  rpcPort?: number;
  gossipHost?: string;
}) {
  const command = "solana-test-validator";
  const solanaArgs = await getSolanaArgs({
    additionalPrograms,
    skipSystemAccounts,
    limitLedgerSize,
    rpcPort,
    gossipHost,
  });

  await killTestValidator();

  await new Promise((r) => setTimeout(r, 1000));

  console.log("Starting test validator...", command);
  await executeCommand({
    command,
    args: [...solanaArgs],
  });
}

export async function killTestValidator() {
  await killProcess("solana-test-validator");
}
