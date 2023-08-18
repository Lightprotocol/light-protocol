import { Command, Args } from "@oclif/core";
import { snakeCaseToCamelCase } from "../../psp-utils/utils";
import { snakeCase } from "snake-case";
import { executeCommandInDir } from "../../psp-utils/process";
import { executeCargoGenerate } from "../../psp-utils/toolchain";

export const PSP_DEFAULT_PROGRAM_ID =
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

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
    let { name } = args;

    this.log("Initializing circom-anchor project...");

    const rustName = snakeCase(name);
    const circomName = snakeCaseToCamelCase(rustName);
    const programName = snakeCaseToCamelCase(rustName, true);

    await executeCargoGenerate({
      args: [
        "generate",
        "--git",
        "https://github.com/Lightprotocol/circom-anchor-template.git",
        "--name",
        name,
        "--define",
        `circom-name=${circomName}`,
        "--define",
        `rust-name=${rustName}`,
        "--define",
        `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
        "--define",
        `anchor-program-name=${programName}`,
      ],
    });

    this.log("Executing yarn install in dir ", name);
    await executeCommandInDir("yarn", ["install"], name);

    this.log("✅ Project initialized successfully");
  }
}
