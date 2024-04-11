import { Command, Flags } from "@oclif/core";
import { initTestEnv } from "../../utils/initTestEnv";
import { CustomLoader } from "../../utils/index";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    "without-indexer": Flags.boolean({
      char: "i",
      description: "Runs a test validator without indexer service.",
      default: false,
    }),
    "without-prover": Flags.boolean({
      char: "p",
      description: "Runs a test validator without prover service.",
      default: false,
    }),
    "skip-system-accounts": Flags.boolean({
      char: "s",
      description:
        "Runs a test validator without initialized light system accounts.",
      default: false,
    }),
    "prove-compressed-accounts": Flags.boolean({
      description: "Enable proving of compressed accounts.",
      default: true,
      exclusive: ["without-prover"],
    }),
    "prove-new-addresses": Flags.boolean({
      description: "Enable proving of new addresses.",
      default: false,
      exclusive: ["without-prover"],
    }),
  };

  async run() {
    const { flags } = await this.parse(SetupCommand);

    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();
    await initTestEnv({
      skipSystemAccounts: flags["skip-system-accounts"],
      indexer: !flags["without-indexer"],
      prover: !flags["without-prover"],
      proveCompressedAccounts: flags["prove-compressed-accounts"],
      proveNewAddresses: flags["prove-new-addresses"],
    });

    this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
  }
}

export default SetupCommand;
