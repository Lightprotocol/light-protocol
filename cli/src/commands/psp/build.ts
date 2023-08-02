import { Args, Command, Flags } from "@oclif/core";
import { buildPSP } from "../../psp-utils/buildPSP";

export default class BuildCommand extends Command {
  static description = "build your PSP";

  static flags = {
    name: Flags.string({ description: "Name of the PSP project." }),
    ptau: Flags.integer({ description: "Ptau value.", default: 15 }),
    circuitDir: Flags.string({
      description: "Directory of the circuit.",
      default: "circuit",
    }),
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
    let { ptau, circuitDir } = flags;
    let { name } = args;

    this.log("building PSP...");
    await buildPSP(circuitDir, ptau, name!);
  }
}
