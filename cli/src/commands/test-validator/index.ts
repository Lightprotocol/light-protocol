import { Command, Flags } from "@oclif/core";
import { sleep } from "@lightprotocol/zk.js";
import { initTestEnv } from "../../utils/initTestEnv";
import { executeCommand } from "../../psp-utils/process";
import { CustomLoader } from "../../utils/index";

class SetupCommand extends Command {
  static description = "Perform setup tasks";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    // TODO(ananas-block): enable pass in of arbitrary bpf programs
    // bpf_program: Flags.string({
    //   aliases: ["bp"],
    //   description:
    //     "Solana bpf program whill be deployed on local test validator <ADDRESS_OR_KEYPAIR> <SBF_PROGRAM.SO>",
    // }),
    // TODO: add this flag
    // kill: Flags.boolean({
    //   aliases: ["k"],
    //   description: "Kills a running test validator.",
    //   hidden: true,
    //   default: true,
    // }),
    background: Flags.boolean({
      char: "b",
      description: "Runs a test validator as a process in the background.",
      default: false,
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
    try {
      if (!flags.background) {
        await initTestEnv({ skip_system_accounts: flags.skip_system_accounts });
      } else {
        initTestEnv({ skip_system_accounts: flags.skip_system_accounts });
        await sleep(10000);
      }
      this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
    } catch (error) {
      this.error(`\nSetup tasks failed: ${error}`);
    }
  }
}

export default SetupCommand;
