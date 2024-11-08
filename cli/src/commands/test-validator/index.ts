import { Command, Flags } from "@oclif/core";
import { initTestEnv, stopTestEnv } from "../../utils/initTestEnv";
import { CustomLoader } from "../../utils/index";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
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
  };

  async run() {
    const { flags } = await this.parse(SetupCommand);
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    if (flags["stop"] === true) {
      await stopTestEnv({
        indexer: !flags["skip-indexer"],
        prover: !flags["skip-prover"],
      });
      this.log("\nTest validator stopped successfully \x1b[32m✔\x1b[0m");
    } else {
      await initTestEnv({
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
      });
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
    }
  }
}

export default SetupCommand;
