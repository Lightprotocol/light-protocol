import { Command, Args } from "@oclif/core";
import {
  ProjectType,
  initRepo,
  cliFlags,
  initFlags,
} from "../../psp-utils/init";

export default class InitCommand extends Command {
  static description = "Initialize circom-anchor project";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the project",
      required: true,
    }),
  };

  static flags = {
    ...cliFlags,
    ...initFlags,
  };

  async run() {
    const { args, flags } = await this.parse(InitCommand);
    const { name } = args;

    this.log("Initializing circom-anchor project...");

    await initRepo(name, ProjectType.CIRCOM, flags);
    this.log("✅ Project initialized successfully");
  }
}
