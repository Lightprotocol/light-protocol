import { Args, Command, Flags } from "@oclif/core";
import { ProjectType, initRepo } from "../../psp-utils/init";

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
        "Whether the main circuit is a circom circuit, not a .light file.",
      default: false,
      required: false,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(InitCommand);
    const { name } = args;

    this.log("🚀 Initializing PSP project...");
    const type = flags.circom ? ProjectType.PSP_CIRCOM : ProjectType.PSP;
    await initRepo(name, type);

    this.log("✅ Project initialized successfully");
  }
}
