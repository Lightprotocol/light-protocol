import { Args, Command, Flags } from "@oclif/core";
import { addCircuit } from "../../psp-utils/addCircuit";
import { initFlags } from "../../psp-utils/init";
export default class InitCommand extends Command {
  static description =
    "Add a circom circuit to your anchor circom or PSP project.";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the circuit",
      required: true,
    }),
  };
  static flags = {
    programName: Flags.string({
      description: "The program the circuit will be verified in.",
      required: true,
    }),
    ...initFlags,
  };
  async run() {
    const { args, flags } = await this.parse(InitCommand);
    const { name } = args;

    this.log("🚀 Adding a circuit...");
    await addCircuit({ name, flags: { ...flags, circom: true } });
    this.log("✅ Project initialized successfully");
  }
}
