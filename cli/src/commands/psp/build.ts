import { Args, Command, Flags } from "@oclif/core";
import { buildFlags, buildPSP } from "../../psp-utils/buildPSP";

export default class BuildCommand extends Command {
  static description = "build your PSP";

  static flags = {
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
    this.log("building PSP...");

    await buildPSP({ ...flags, programName: name! });
  }
}
