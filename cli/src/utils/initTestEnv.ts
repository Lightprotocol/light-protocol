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
import { killProver, startProver } from "./processProverServer";
import { killIndexer, startIndexer } from "./processPhotonIndexer";

type Program = { id: string; name?: string; tag?: string; path?: string };
export const SYSTEM_PROGRAMS: Program[] = [
  {
    id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
    name: "spl_noop.so",
    tag: SPL_NOOP_PROGRAM_TAG,
  },
  {
    id: "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7",
    name: "light_system_program_pinocchio.so",
    tag: LIGHT_SYSTEM_PROGRAM_TAG,
  },
  {
    id: "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
    name: "light_compressed_token.so",
    tag: LIGHT_COMPRESSED_TOKEN_TAG,
  },
  {
    id: "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq",
    name: "account_compression.so",
    tag: LIGHT_ACCOUNT_COMPRESSION_TAG,
  },
  {
    id: "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX",
    name: "light_registry.so",
    tag: LIGHT_REGISTRY_TAG,
  },
];

export async function stopTestEnv(options: {
  indexer: boolean;
  prover: boolean;
}) {
  const processesToKill = [
    { name: "photon", condition: options.indexer, killFunction: killIndexer },
    { name: "prover", condition: options.prover, killFunction: killProver },
    {
      name: "test-validator",
      condition: true,
      killFunction: killTestValidator,
    },
  ];
  const killPromises = processesToKill
    .filter((process) => process.condition)
    .map(async (process) => {
      try {
        if (process.killFunction) {
          await process.killFunction();
        }
        console.log(`${process.name} stopped successfully.`);
      } catch (error) {
        console.error(`Failed to stop ${process.name}:`, error);
      }
    });

  await Promise.all(killPromises);

  console.log("All specified processes and validator stopped.");
}

export async function initTestEnv({
  additionalPrograms,
  skipSystemAccounts,
  indexer = true,
  prover = true,
  rpcPort = 8899,
  indexerPort = 8784,
  grpcPort = 50051,
  proverPort = 3001,
  gossipHost = "127.0.0.1",
  checkPhotonVersion = true,
  photonDatabaseUrl,
  limitLedgerSize,
  geyserConfig,
  validatorArgs,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  indexer: boolean;
  prover: boolean;
  rpcPort?: number;
  indexerPort?: number;
  grpcPort?: number;
  proverPort?: number;
  gossipHost?: string;
  checkPhotonVersion?: boolean;
  photonDatabaseUrl?: string;
  limitLedgerSize?: number;
  validatorArgs?: string;
  geyserConfig?: string;
}) {
  // We cannot await this promise directly because it will hang the process
  startTestValidator({
    additionalPrograms,
    skipSystemAccounts,
    limitLedgerSize,
    rpcPort,
    gossipHost,
    validatorArgs,
    geyserConfig,
  });
  await waitForServers([{ port: rpcPort, path: "/health" }]);
  await confirmServerStability(`http://127.0.0.1:${rpcPort}/health`);

  if (indexer) {
    const config = getConfig();
    config.indexerUrl = `http://127.0.0.1:${indexerPort}`;
    setConfig(config);
    await startIndexer(
      `http://127.0.0.1:${rpcPort}`,
      indexerPort,
      grpcPort,
      checkPhotonVersion,
      photonDatabaseUrl,
    );
  }

  if (prover) {
    const config = getConfig();
    config.proverUrl = `http://127.0.0.1:${proverPort}`;
    setConfig(config);
    try {
      // TODO: check if using redisUrl is better here.
      await startProver(proverPort);
    } catch (error) {
      console.error("Failed to start prover:", error);
      // Prover logs will be automatically displayed by spawnBinary in process.ts
      throw error;
    }
  }
}

export async function initTestEnvIfNeeded({
  additionalPrograms,
  skipSystemAccounts,
  indexer = false,
  prover = false,
  geyserConfig,
  validatorArgs,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  indexer?: boolean;
  prover?: boolean;
  geyserConfig?: string;
  validatorArgs?: string;
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
      geyserConfig,
      validatorArgs,
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
export function programsDirPath(): string {
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
export function programFilePath(programName: string): string {
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
  // TODO: adjust program tags
  const programs: Program[] = [...SYSTEM_PROGRAMS];
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
  validatorArgs,
  geyserConfig,
}: {
  additionalPrograms?: { address: string; path: string }[];
  skipSystemAccounts?: boolean;
  limitLedgerSize?: number;
  rpcPort?: number;
  gossipHost?: string;
  validatorArgs?: string;
  geyserConfig?: string;
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

  // Add geyser config if provided
  if (geyserConfig) {
    solanaArgs.push("--geyser-plugin-config", geyserConfig);
  }

  // Add custom validator args last
  if (validatorArgs) {
    solanaArgs.push(...validatorArgs.split(" "));
  }
  console.log("Starting test validator...");
  await executeCommand({
    command,
    args: [...solanaArgs],
  });
}

export async function killTestValidator() {
  await killProcess("solana-test-validator");
}
