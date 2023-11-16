import { Args, Command, Flags } from "@oclif/core";
import { cliFlags, initFlags } from "../../psp-utils/init";
import { addProgram } from "../../psp-utils/addProgram";

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
    ...cliFlags,
    ...initFlags,
  };

  async run() {
    const { args, flags } = await this.parse(InitCommand);
    const { name } = args;

    this.log("Adding Program...");
    await addProgram({ name, flags });

    this.log("✅ Program initialized successfully");
  }
}
