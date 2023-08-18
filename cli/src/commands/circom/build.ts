import { Args, Command, Flags } from "@oclif/core";
import { buildCircom } from "../../psp-utils/buildCircom";

export default class BuildCommand extends Command {
  static description = "Build circom-anchor project";

  static flags = {
    name: Flags.string({ description: "Name of the circom-anchor project." }),
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
      description: "The name of the circom-anchor project.",
      required: true,
    }),
  };
  async run() {
    const { flags, args } = await this.parse(BuildCommand);
    let { ptau, circuitDir } = flags;
    let { name } = args;

    this.log("Building circom-anchor project...");
    await buildCircom(circuitDir, ptau, name!);
  }
}
