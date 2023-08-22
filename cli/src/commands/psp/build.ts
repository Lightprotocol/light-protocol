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
    onlyCircuit: Flags.boolean({
      description: "Directory of the circuit.",
      default: false,
      required: false,
    }),
    skipCircuit: Flags.boolean({
      description: "Directory of the circuit.",
      default: false,
      required: false,
    }),
    skipMacroCircom: Flags.boolean({
      description: "Directory of the circuit.",
      default: false,
      required: false,
    }),
    addCircuitName: Flags.string({
      description:
        "Name of circuit main file, the name has to be camel case and include the suffix Main.",
      required: false,
      parse: async (circuitName: string) => {
        if (!isCamelCase(circuitName))
          throw new Error(
            `Circuit name must be camel case. ${circuitName} is not valid.`
          );
        return circuitName;
      },
      multiple: true,
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
    let { name } = args;
    console.log("build psp flags", flags);
    this.log("building PSP...");
    await buildPSP({ ...flags, programName: name! });
  }
}

function isCamelCase(str: string): boolean {
  return /^[a-z]+([A-Z][a-z0-9]*)*$/.test(str);
}
