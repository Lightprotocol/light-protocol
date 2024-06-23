import { Command, Flags } from "@oclif/core";
import { CustomLoader, startProver } from "../../utils/index";

class StartProver extends Command {
  static description = "Start gnark prover";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    "skip-prove-compressed-accounts": Flags.boolean({
      description: "Skip proving of compressed accounts.",
      default: false,
      char: "c",
    }),
    "skip-prove-new-addresses": Flags.boolean({
      description: "Skip proving of new addresses.",
      default: false,
      char: "n",
    }),
    "prover-port": Flags.integer({
      description: "Enable Light Prover server on this port.",
      required: false,
      default: 3001,
    }),
  };

  async run() {
    const { flags } = await this.parse(StartProver);
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    await startProver(
      flags["prover-port"],
      !flags["skip-prove-compressed-accounts"],
      !flags["skip-prove-new-addresses"],
    );
    this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
  }
}

export default StartProver;
