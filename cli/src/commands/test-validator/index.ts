import { Command, Flags } from "@oclif/core";
import {
  initTestEnv,
  stopTestEnv,
  SYSTEM_PROGRAMS,
} from "../../utils/initTestEnv";
import { CustomLoader } from "../../utils/index";
import path from "path";
import fs from "fs";
import { featureFlags } from "@lightprotocol/stateless.js";

class SetupCommand extends Command {
  static description =
    "Start a local test setup with: Solana test validator, Photon indexer, and Light prover";

  static examples = [
    "$ light test-validator",
    "$ light test-validator --skip-indexer",
    "$ light test-validator --geyser-config ./config.json",
    '$ light test-validator --validator-args "--limit-ledger-size 50000000"',
    "$ light test-validator --sbf-program <address> <path/program>",
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
    "prover-run-mode": Flags.string({
      description:
        "Specify the running mode for the prover (forester, forester-test, rpc, or full)",
      options: [
        "rpc",
        "forester",
        "forester-test",
        "full",
        "full-test",
      ] as const,
      required: false,
      exclusive: ["skip-prover"],
    }),
    circuit: Flags.string({
      description: "Specify individual circuits to enable.",
      options: [
        "inclusion",
        "non-inclusion",
        "combined",
        "append",
        "update",
        "append-test",
        "update-test",
      ],
      multiple: true,
      required: false,
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
    version: Flags.string({
      description: "Specify which version to use (V1 or V2)",
      options: ["V1", "V2"] as const,
      required: false,
      default: "V2",
    }),
  };

  validatePrograms(programs: { address: string; path: string }[]): void {
    // Check for duplicate addresses among provided programs
    const addresses = new Set<string>();
    for (const program of programs) {
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

    if (flags["version"]) {
      featureFlags.version = flags["version"] as "V1" | "V2";
      this.log(`Using ${featureFlags.version} configuration`);
    }

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
      });
      this.log("\nTest validator stopped successfully \x1b[32m✔\x1b[0m");
    } else {
      const rawValues = flags["sbf-program"] || [];

      if (rawValues.length % 2 !== 0) {
        this.error("Each --sbf-program flag must have exactly two arguments");
      }

      const programs: { address: string; path: string }[] = [];
      for (let i = 0; i < rawValues.length; i += 2) {
        programs.push({
          address: rawValues[i],
          path: rawValues[i + 1],
        });
      }

      this.validatePrograms(programs);

      await initTestEnv({
        additionalPrograms: programs,
        checkPhotonVersion: !flags["relax-indexer-version-constraint"],
        indexer: !flags["skip-indexer"],
        limitLedgerSize: flags["limit-ledger-size"],
        photonDatabaseUrl: flags["indexer-db-url"],
        rpcPort: flags["rpc-port"],
        gossipHost: flags["gossip-host"],
        indexerPort: flags["indexer-port"],
        proverPort: flags["prover-port"],
        prover: !flags["skip-prover"],
        skipSystemAccounts: flags["skip-system-accounts"],
        proverRunMode: flags["prover-run-mode"] as
          | "inclusion"
          | "non-inclusion"
          | "forester"
          | "forester-test"
          | "rpc"
          | "full"
          | "full-test"
          | undefined,
        circuits: flags["circuit"],
        geyserConfig: flags["geyser-config"],
        validatorArgs: flags["validator-args"],
      });
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
    }
  }
}

export default SetupCommand;
