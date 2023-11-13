import { Args, Command, Flags } from "@oclif/core";
import { buildFlags, buildPSP } from "../../psp-utils/buildPSP";

export default class BuildCommand extends Command {
  static description = "build your PSP";

  static flags = {
    ...buildFlags,
    skipLinkCircuitlib: Flags.boolean({
      description: "Omits the linking of the circuit-lib library.",
      required: false,
      default: false,
    }),
    // TODO: pass along anchor build options // execsync thingy alt.
  };

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the PSP project.",
      required: false,
    }),
  };
  async run() {
    const { flags, args } = await this.parse(BuildCommand);
    const { name } = args;
    this.log("building PSP...");

    await buildPSP({ ...flags, programName: name });
  }
}
