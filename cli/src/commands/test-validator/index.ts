import { Command, Flags } from "@oclif/core";
import { initTestEnv } from "../../utils/initTestEnv";
import { CustomLoader } from "../../utils/index";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    photon: Flags.boolean({
      char: "p",
      description: "Runs a test validator with photon indexer.",
      default: true,
    }),
    skip_system_accounts: Flags.boolean({
      char: "s",
      description:
        "Runs a test validator without initialized light system accounts.",
      default: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(SetupCommand);

    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();
    await initTestEnv({
      skipSystemAccounts: flags.skip_system_accounts,
      photon: flags.photon,
    });

    this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
  }
}

export default SetupCommand;
