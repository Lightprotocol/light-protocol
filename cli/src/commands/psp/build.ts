import { Args, Command, Flags } from "@oclif/core";
import { buildFlags, buildPSP } from "../../psp-utils/buildPSP";

export default class BuildCommand extends Command {
  static description = "build your PSP";

  static flags = {
    circom: Flags.boolean({
      description:
        "Whether the main circuit is a circom circuit not a .light file.",
      default: false,
      required: false,
    }),
    ...buildFlags,
    // TODO: pass along anchor build options // execsync thingy alt.
  };

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the PSP project.",
      required: true,
    }),
  };
  async run() {
    const { flags, args } = await this.parse(BuildCommand);
    let { name } = args;
    console.log("build psp flags", flags);
    this.log("building PSP...");

    await buildPSP({ ...flags, programName: name! });
  }
}
