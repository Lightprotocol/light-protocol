import { Command, Args } from "@oclif/core";
import { execSync } from "child_process";
import { downloadFileIfNotExists } from "../../utils";
import { CustomLoader } from "../../utils/utils";
const path = require("path");

class InitCommand extends Command {
  static description = "Initialize a PSP project";

  static examples = ["light init <project-name>"];

  static args = {
    name: Args.string({
      name: "name",
      description: "the name of your project",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { args } = await this.parse(InitCommand);
    const { name } = args;

    if (!name) {
      this.error("Project name is undefined. Please provide a project name.", {
        exit: 0,
      });
    }

    const loader = new CustomLoader("Initializing PSP...");

    loader.start();

    try {
      const anchorPath = path.resolve(__dirname, "../../bin/light-anchor");
      const dirPath = path.resolve(__dirname, "../../bin/");

      await downloadFileIfNotExists({
        filePath: anchorPath,
        dirPath,
        repoName: "anchor",
        fileName: "light-anchor",
      });

      execSync(`${anchorPath} init-psp ${name}`);
      this.log("Inited successfully");
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`${error}`);
    }
  }
}

export default InitCommand;
