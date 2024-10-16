import { Command, Flags } from "@oclif/core";
import { CustomLoader, startProver } from "../../utils/index";

class StartProver extends Command {
  static description = "Start gnark prover";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static flags = {
    "prover-port": Flags.integer({
      description: "Enable Light Prover server on this port.",
      required: false,
      default: 3001,
    }),
    "run-mode": Flags.string({
      description:
        "Specify the running mode (forester, forester-test, rpc, full, or full-test)",
      options: ["rpc", "forester", "forester-test", "full", "full-test"],
      required: false,
    }),
    circuit: Flags.string({
      description: "Specify individual circuits to enable.",
      options: [
        "inclusion",
        "non-inclusion",
        "combined",
        "append",
        "append2",
        "update",
        "append-test",
        "append2-test",
        "update-test",
      ],
      multiple: true,
      required: false,
    }),
  };

  async run() {
    const { flags } = await this.parse(StartProver);
    const loader = new CustomLoader("Performing setup tasks...\n");
    loader.start();

    if (!flags["run-mode"] && !flags["circuit"]) {
      this.log("Please specify --run-mode or --circuit.");
      return;
    }

    await startProver(
      flags["prover-port"],
      flags["run-mode"],
      flags["circuit"],
    );
    this.log("\nSetup tasks completed successfully \x1b[32m✔\x1b[0m");
  }
}

export default StartProver;
