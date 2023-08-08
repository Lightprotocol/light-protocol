import { Args, Command } from "@oclif/core";
import { snakeCase } from "snake-case";
import { downloadCargoGenerateIfNotExists } from "../../psp-utils/download";
import { executeCommandInDir } from "../../psp-utils/process";
import { executeCargoGenerate } from "../../psp-utils/toolchain";
import * as path from "path";
import { PSP_TEMPLATE_TAG } from "../../psp-utils/contants";

export const PSP_DEFAULT_PROGRAM_ID =
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

export default class InitCommand extends Command {
  static description = "Initialize a PSP project.";

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

    this.log("🚀 Initializing PSP project...");
    const cargoGeneratePath = path.resolve(
      __dirname,
      "../../bin/cargo-generate"
    );
    const dirPath = path.resolve(__dirname, "../../bin/");

    await downloadCargoGenerateIfNotExists({
      localFilePath: cargoGeneratePath,
      dirPath,
    });

    const circomName = snakeCase(name);
    const rustName = snakeCase(name);
    await executeCargoGenerate({
      args: [
        "generate",
        "--git",
        "https://github.com/Lightprotocol/psp-template",
        "--branch",
        PSP_TEMPLATE_TAG,
        "--name",
        name,
        "--define",
        `circom-name=${circomName}`,
        "--define",
        `rust-name=${rustName}`,
        "--define",
        `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
      ],
    });
    await executeCommandInDir("yarn", ["install"], name);

    this.log("✅ Project initialized successfully");
  }
}
