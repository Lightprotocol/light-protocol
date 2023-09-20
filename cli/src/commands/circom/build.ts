import { Args, Command } from "@oclif/core";
import { buildFlags, buildPSP } from "../../psp-utils";

export default class BuildCommand extends Command {
  static description = "Build circom-anchor project";

  static flags = {
    ...buildFlags,
    // TODO: pass along anchor build options // execsync thingy alt.
  };
  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the circom-anchor project.",
      required: true,
    }),
  };
  async run() {
    const { flags, args } = await this.parse(BuildCommand);
    let { name } = args;

    this.log("Building circom-anchor project...");
    await buildPSP({ ...flags, programName: name!, skipLinkCircuitlib: true });
  }
}
