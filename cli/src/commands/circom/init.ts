import { Command, Args } from "@oclif/core";
import { ProjectType, initRepo } from "../../psp-utils/init";

export default class InitCommand extends Command {
  static description = "Initialize circom-anchor project";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the project",
      required: true,
    }),
  };

  async run() {
    const { args } = await this.parse(InitCommand);
    const { name } = args;

    this.log("Initializing circom-anchor project...");

    await initRepo(name, ProjectType.CIRCOM);
    this.log("✅ Project initialized successfully");
  }
}
