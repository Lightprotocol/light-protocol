import { Args, Command, Flags } from "@oclif/core";
import { addCircuit } from "../../psp-utils/addCircuit";

export default class InitCommand extends Command {
  static description = "Initialize a PSP project.";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the project",
      required: true,
    }),
  };
  static flags = {
    circom: Flags.boolean({
      description:
        "Whether the main circuit is a circom circuit not a .light file.",
      default: false,
      required: false,
    }),
  };

  async run() {
    const { flags, args } = await this.parse(InitCommand);
    let { name } = args;

    this.log("🚀 Initializing PSP project...");

    addCircuit({ name, ...flags });
    this.log("✅ Project initialized successfully");
  }
}
