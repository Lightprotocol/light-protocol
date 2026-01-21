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
  confirmRpcReadiness,
  confirmServerStability,
  executeCommand,
  killProcess,
  waitForServers,
} from "./process";
import { killProver, startProver } from "./processProverServer";
import { killIndexer, startIndexer } from "./processPhotonIndexer";
import { Connection, PublicKey } from "@solana/web3.js";

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

// Programs to clone from devnet/mainnet (the three core Light programs)
const PROGRAMS_TO_CLONE = [
  "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX", // Light Registry
  "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7", // Light System Program
  "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq", // Account Compression
];

// Known Light Registry accounts to clone (excludes forester/epoch accounts)
// These are the core config accounts needed for protocol operation
const REGISTRY_ACCOUNTS_TO_CLONE = [
  "CuEtcKkkbTn6qy2qxqDswq5U2ADsqoipYDAYfRvxPjcp", // governance_authority_pda (ProtocolConfigPda)
  "8gH9tmziWsS8Wc4fnoN5ax3jsSumNYoRDuSBvmH2GMH8", // config_counter_pda
  "35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh", // registered_program_pda
  "DumMsyvkaGJG4QnQ1BhTgvoRMXsgGxfpKDUCr22Xqu4w", // registered_registry_program_pda
  "24rt4RgeyjUCWGS2eF7L7gyNMuz6JWdqYpAvb1KRoHxs", // group_pda
];

/**
 * Fetches account public keys owned by a program from a given cluster.
 * For Light Registry, returns known config accounts (skips forester/epoch accounts).
 */
async function getProgramOwnedAccounts(
  programId: string,
  rpcUrl: string,
): Promise<string[]> {
  const isRegistry =
    programId === "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

  if (isRegistry) {
    // Return known registry accounts instead of fetching all (too slow due to 88k+ forester accounts)
    return REGISTRY_ACCOUNTS_TO_CLONE;
  } else {
    // For other programs, fetch all accounts
    const connection = new Connection(rpcUrl);
    const accounts = await connection.getProgramAccounts(
      new PublicKey(programId),
      { dataSlice: { offset: 0, length: 0 } },
    );
    return accounts.map((acc) => acc.pubkey.toBase58());
  }
}

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
  upgradeablePrograms,
  skipSystemAccounts,
  indexer = true,
  prover = true,
  rpcPort = 8899,
  indexerPort = 8784,
  proverPort = 3001,
  gossipHost = "127.0.0.1",
  checkPhotonVersion = true,
  photonDatabaseUrl,
  limitLedgerSize,
  geyserConfig,
  validatorArgs,
  cloneNetwork,
  verbose,
  skipReset,
}: {
  additionalPrograms?: { address: string; path: string }[];
  upgradeablePrograms?: {
    address: string;
    path: string;
    upgradeAuthority: string;
  }[];
  skipSystemAccounts?: boolean;
  indexer: boolean;
  prover: boolean;
  rpcPort?: number;
  indexerPort?: number;
  proverPort?: number;
  gossipHost?: string;
  checkPhotonVersion?: boolean;
  photonDatabaseUrl?: string;
  limitLedgerSize?: number;
  validatorArgs?: string;
  geyserConfig?: string;
  cloneNetwork?: "devnet" | "mainnet";
  verbose?: boolean;
  skipReset?: boolean;
}) {
  // We cannot await this promise directly because it will hang the process
  startTestValidator({
    additionalPrograms,
    upgradeablePrograms,
    skipSystemAccounts,
    limitLedgerSize,
    rpcPort,
    gossipHost,
    validatorArgs,
    geyserConfig,
    cloneNetwork,
    verbose,
    skipReset,
  });
  await waitForServers([{ port: rpcPort, path: "/health" }]);
  await confirmServerStability(`http://127.0.0.1:${rpcPort}/health`);
  await confirmRpcReadiness(`http://127.0.0.1:${rpcPort}`);

  if (prover) {
    const config = getConfig();
    config.proverUrl = `http://127.0.0.1:${proverPort}`;
    setConfig(config);
    try {
      await startProver(proverPort);
    } catch (error) {
      console.error("Failed to start prover:", error);
      throw error;
    }
  }

  if (indexer) {
    const config = getConfig();
    config.indexerUrl = `http://127.0.0.1:${indexerPort}`;
    setConfig(config);
    const proverUrlForIndexer = prover
      ? `http://127.0.0.1:${proverPort}`
      : undefined;
    await startIndexer(
      `http://127.0.0.1:${rpcPort}`,
      indexerPort,
      checkPhotonVersion,
      photonDatabaseUrl,
      proverUrlForIndexer,
    );
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
  upgradeablePrograms,
  skipSystemAccounts,
  limitLedgerSize,
  rpcPort,
  gossipHost,
  downloadBinaries = true,
  cloneNetwork,
  verbose = false,
  skipReset = false,
}: {
  additionalPrograms?: { address: string; path: string }[];
  upgradeablePrograms?: {
    address: string;
    path: string;
    upgradeAuthority: string;
  }[];
  skipSystemAccounts?: boolean;
  limitLedgerSize?: number;
  rpcPort?: number;
  gossipHost?: string;
  downloadBinaries?: boolean;
  cloneNetwork?: "devnet" | "mainnet";
  verbose?: boolean;
  skipReset?: boolean;
}): Promise<Array<string>> {
  const dirPath = programsDirPath();

  const solanaArgs = [
    `--limit-ledger-size=${limitLedgerSize}`,
    `--rpc-port=${rpcPort}`,
    `--bind-address=${gossipHost}`,
    "--quiet",
  ];

  if (!skipReset) {
    solanaArgs.unshift("--reset");
  }

  // Add cluster URL if cloning from a network
  if (cloneNetwork) {
    const clusterUrl = cloneNetwork === "devnet" ? "devnet" : "mainnet-beta";
    solanaArgs.push("--url", clusterUrl);
  }

  // Process system programs
  for (const program of SYSTEM_PROGRAMS) {
    const shouldClone = cloneNetwork && PROGRAMS_TO_CLONE.includes(program.id);

    if (shouldClone) {
      // Clone program from network
      solanaArgs.push("--clone-upgradeable-program", program.id);
    } else {
      // Load program from local binary
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

  // Clone all accounts owned by the programs being cloned
  if (cloneNetwork) {
    const rpcUrl =
      cloneNetwork === "devnet"
        ? "https://api.devnet.solana.com"
        : "https://api.mainnet-beta.solana.com";

    for (const programId of PROGRAMS_TO_CLONE) {
      if (verbose) {
        console.log(`Fetching accounts owned by ${programId}...`);
      }
      const accounts = await getProgramOwnedAccounts(programId, rpcUrl);
      if (verbose) {
        console.log(`Found ${accounts.length} accounts`);
      }
      for (const account of accounts) {
        solanaArgs.push("--maybe-clone", account);
      }
    }
  }

  // Add additional user-provided programs (always loaded locally)
  if (additionalPrograms) {
    for (const program of additionalPrograms) {
      solanaArgs.push("--bpf-program", program.address, program.path);
    }
  }

  // Add upgradeable programs (with upgrade authority)
  if (upgradeablePrograms) {
    for (const program of upgradeablePrograms) {
      solanaArgs.push(
        "--upgradeable-program",
        program.address,
        program.path,
        program.upgradeAuthority,
      );
    }
  }

  // Load local system accounts only if not cloning from network
  if (!skipSystemAccounts && !cloneNetwork) {
    const accountsRelPath = "../../accounts";
    const accountsPath = path.resolve(__dirname, accountsRelPath);
    solanaArgs.push("--account-dir", accountsPath);
  }

  return solanaArgs;
}

export async function startTestValidator({
  additionalPrograms,
  upgradeablePrograms,
  skipSystemAccounts,
  limitLedgerSize,
  rpcPort,
  gossipHost,
  validatorArgs,
  geyserConfig,
  cloneNetwork,
  verbose,
  skipReset,
}: {
  additionalPrograms?: { address: string; path: string }[];
  upgradeablePrograms?: {
    address: string;
    path: string;
    upgradeAuthority: string;
  }[];
  skipSystemAccounts?: boolean;
  limitLedgerSize?: number;
  rpcPort?: number;
  gossipHost?: string;
  validatorArgs?: string;
  geyserConfig?: string;
  cloneNetwork?: "devnet" | "mainnet";
  verbose?: boolean;
  skipReset?: boolean;
}) {
  const command = "solana-test-validator";
  const solanaArgs = await getSolanaArgs({
    additionalPrograms,
    upgradeablePrograms,
    skipSystemAccounts,
    limitLedgerSize,
    rpcPort,
    gossipHost,
    cloneNetwork,
    verbose,
    skipReset,
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
