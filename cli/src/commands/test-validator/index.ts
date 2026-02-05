import { Command, Flags } from "@oclif/core";
import { initTestEnv, stopTestEnv } from "../../utils/initTestEnv";
import {
  CustomLoader,
  LIGHT_ACCOUNT_COMPRESSION_TAG,
  LIGHT_COMPRESSED_TOKEN_TAG,
  LIGHT_REGISTRY_TAG,
  LIGHT_SYSTEM_PROGRAM_TAG,
  SPL_NOOP_PROGRAM_TAG,
} from "../../utils/index";
import path from "path";
import fs from "fs";

class SetupCommand extends Command {
  static description =
    "Start a local test setup with: Solana test validator, Photon indexer, and Light prover";

  static examples = [
    "$ light test-validator",
    "$ light test-validator --skip-indexer",
    "$ light test-validator --geyser-config ./config.json",
    '$ light test-validator --validator-args "--limit-ledger-size 50000000"',
    "$ light test-validator --sbf-program <address> <path/program>",
    "$ light test-validator --upgradeable-program <address> <path/program> <upgrade_authority>",
    "$ light test-validator --devnet",
    "$ light test-validator --mainnet",
  ];

  protected finally(err: Error | undefined): Promise<any> {
    if (err) {
      console.error(err);
    }
    process.exit();
  }

  static flags = {
    "skip-indexer": Flags.boolean({
      description: "Runs a test validator without starting a new indexer.",
      default: false,
    }),
    "skip-prover": Flags.boolean({
      description:
        "Runs a test validator without starting a new prover service.",
      default: false,
    }),
    forester: Flags.boolean({
      description:
        "Start the forester service for auto-compression of compressible accounts.",
      default: false,
    }),
    "forester-port": Flags.integer({
      description: "Port for the forester API server.",
      required: false,
      default: 8080,
    }),
    "compressible-pda-program": Flags.string({
      description:
        "Compressible PDA programs to track. Format: 'program_id:discriminator_base58'. Can be specified multiple times.",
      required: false,
      multiple: true,
    }),
    "skip-system-accounts": Flags.boolean({
      description:
        "Runs a test validator without initialized light system accounts.",
      default: false,
    }),
    "relax-indexer-version-constraint": Flags.boolean({
      description:
        "Disables indexer version check. Only use if you know what you are doing.",
      default: false,
      exclusive: ["skip-indexer"],
    }),
    "indexer-db-url": Flags.string({
      description:
        "Custom indexer database URL to store indexing data. By default we use an in-memory SQLite database.",
      required: false,
      exclusive: ["skip-indexer"],
    }),
    "rpc-port": Flags.integer({
      description:
        "Enable JSON RPC on this port, and the next port for the RPC websocket.",
      required: false,
      default: 8899,
    }),
    "indexer-port": Flags.integer({
      description: "Enable Photon indexer on this port.",
      required: false,
      default: 8784,
      exclusive: ["skip-indexer"],
    }),
    "prover-port": Flags.integer({
      description: "Enable Light Prover server on this port.",
      required: false,
      default: 3001,
      exclusive: ["skip-prover"],
    }),
    "limit-ledger-size": Flags.integer({
      description: "Keep this amount of shreds in root slots.",
      required: false,
      default: 10000,
    }),
    "gossip-host": Flags.string({
      description:
        "Gossip DNS name or IP address for the validator to advertise in gossip.",
      required: false,
      default: "127.0.0.1",
    }),
    stop: Flags.boolean({
      description:
        "Stops the test validator and dependent processes. Use with --skip-indexer, --skip-prover to keep specific services running.",
      required: false,
      default: false,
    }),
    "geyser-config": Flags.string({
      description: "Path to Geyser plugin config.",
      required: false,
    }),
    "validator-args": Flags.string({
      description:
        "Additional arguments to pass directly to solana-test-validator. Only use if you know what you are doing.",
      required: false,
      exclusive: ["geyser-config"],
    }),
    "sbf-program": Flags.string({
      description:
        "Add a SBF program to the genesis configuration with upgrades disabled. If the ledger already exists then this parameter is silently ignored. First argument can be a pubkey string or path to a keypair",
      required: false,
      multiple: true,
      summary: "Usage: --sbf-program <address> <path/program_name.so>",
    }),
    "upgradeable-program": Flags.string({
      description:
        "Add an upgradeable SBF program to the genesis configuration. Required for programs that need compressible config initialization. If the ledger already exists then this parameter is silently ignored.",
      required: false,
      multiple: true,
      summary:
        "Usage: --upgradeable-program <address> <path/program_name.so> <upgrade_authority>",
    }),
    devnet: Flags.boolean({
      description:
        "Clone Light Protocol programs and accounts from devnet instead of loading local binaries.",
      default: false,
      exclusive: ["mainnet"],
    }),
    mainnet: Flags.boolean({
      description:
        "Clone Light Protocol programs and accounts from mainnet instead of loading local binaries.",
      default: false,
      exclusive: ["devnet"],
    }),
    verbose: Flags.boolean({
      char: "v",
      description: "Enable verbose logging.",
      default: false,
    }),
    "skip-reset": Flags.boolean({
      description: "Skip resetting the ledger.",
      default: false,
    }),
    "use-surfpool": Flags.boolean({
      description:
        "Use surfpool instead of solana-test-validator (default). Pass --no-use-surfpool to use solana-test-validator.",
      default: true,
      allowNo: true,
    }),
    "account-dir": Flags.string({
      description:
        "Additional directory containing account JSON files to preload. Can be specified multiple times.",
      required: false,
      multiple: true,
      summary: "Usage: --account-dir <path/to/accounts/>",
    }),
  };

  validatePrograms(
    programs: { address: string; path: string }[],
    upgradeablePrograms: {
      address: string;
      path: string;
      upgradeAuthority: string;
    }[],
  ): void {
    // Check for duplicate addresses among all provided programs
    const addresses = new Set<string>();
    const allPrograms = [
      ...programs.map((p) => ({ ...p, type: "sbf" })),
      ...upgradeablePrograms.map((p) => ({ ...p, type: "upgradeable" })),
    ];

    for (const program of allPrograms) {
      if (addresses.has(program.address)) {
        this.error(`Duplicate program address detected: ${program.address}`);
      }
      addresses.add(program.address);

      // Get the program filename from the path
      const programFileName = path.basename(program.path);

      // Check for collisions with system programs (both address and filename)
      const systemProgramCollision = SYSTEM_PROGRAMS.find(
        (sysProg) =>
          sysProg.id === program.address ||
          (sysProg.name && programFileName === sysProg.name),
      );

      if (systemProgramCollision) {
        const collisionType =
          systemProgramCollision.id === program.address
            ? `address (${program.address})`
            : `filename (${programFileName})`;

        this.error(
          `Program ${collisionType} collides with system program ` +
            `"${systemProgramCollision.name || systemProgramCollision.id}". ` +
            `System programs cannot be overwritten.`,
        );
      }

      // Validate program file exists
      const programPath = path.resolve(program.path);
      if (!fs.existsSync(programPath)) {
        this.error(`Program file not found: ${programPath}`);
      }
    }
  }

  async run() {
    const { flags } = await this.parse(SetupCommand);
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    if (flags["geyser-config"]) {
      const configPath = path.resolve(flags["geyser-config"]);
      if (!fs.existsSync(configPath)) {
        this.error(`Geyser config file not found: ${configPath}`);
      }
    }
    if (flags["stop"] === true) {
      await stopTestEnv({
        indexer: !flags["skip-indexer"],
        prover: !flags["skip-prover"],
        forester: flags.forester,
      });
      this.log("\nTest validator stopped successfully \x1b[32m✔\x1b[0m");
    } else {
      // Parse --sbf-program flags (2 arguments each: address, path)
      const rawSbfValues = flags["sbf-program"] || [];
      if (rawSbfValues.length % 2 !== 0) {
        this.error("Each --sbf-program flag must have exactly two arguments");
      }

      const programs: { address: string; path: string }[] = [];
      for (let i = 0; i < rawSbfValues.length; i += 2) {
        programs.push({
          address: rawSbfValues[i],
          path: rawSbfValues[i + 1],
        });
      }

      // Parse --upgradeable-program flags (3 arguments each: address, path, upgrade_authority)
      const rawUpgradeableValues = flags["upgradeable-program"] || [];
      if (rawUpgradeableValues.length % 3 !== 0) {
        this.error(
          "Each --upgradeable-program flag must have exactly three arguments: <address> <path> <upgrade_authority>",
        );
      }

      const upgradeablePrograms: {
        address: string;
        path: string;
        upgradeAuthority: string;
      }[] = [];
      for (let i = 0; i < rawUpgradeableValues.length; i += 3) {
        upgradeablePrograms.push({
          address: rawUpgradeableValues[i],
          path: rawUpgradeableValues[i + 1],
          upgradeAuthority: rawUpgradeableValues[i + 2],
        });
      }

      this.validatePrograms(programs, upgradeablePrograms);

      await initTestEnv({
        additionalPrograms: programs,
        upgradeablePrograms: upgradeablePrograms,
        checkPhotonVersion: !flags["relax-indexer-version-constraint"],
        indexer: !flags["skip-indexer"],
        limitLedgerSize: flags["limit-ledger-size"],
        photonDatabaseUrl: flags["indexer-db-url"],
        rpcPort: flags["rpc-port"],
        gossipHost: flags["gossip-host"],
        indexerPort: flags["indexer-port"],
        proverPort: flags["prover-port"],
        prover: !flags["skip-prover"],
        forester: flags.forester,
        foresterPort: flags["forester-port"],
        compressiblePdaPrograms: flags["compressible-pda-program"],
        skipSystemAccounts: flags["skip-system-accounts"],
        geyserConfig: flags["geyser-config"],
        validatorArgs: flags["validator-args"],
        cloneNetwork: flags.devnet
          ? "devnet"
          : flags.mainnet
            ? "mainnet"
            : undefined,
        verbose: flags.verbose,
        skipReset: flags["skip-reset"],
        useSurfpool: flags["use-surfpool"],
        additionalAccountDirs: flags["account-dir"],
      });
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
    }
  }
}

export default SetupCommand;

export const SYSTEM_PROGRAMS = [
  {
    id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
    name: "spl_noop.so",
    tag: SPL_NOOP_PROGRAM_TAG,
  },
  {
    id: "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7",
    name: "light_system_program.so",
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
