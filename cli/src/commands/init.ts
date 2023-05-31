import type { Arguments, CommandBuilder } from "yargs";
import { Options } from "yargs-parser";
import { snakeCase } from "snake-case";
import { downloadCargoGenerateIfNotExists } from "../utils/download";
import { executeCommandInDir } from "../utils/process";
import { executeCargoGenerate } from "../utils/toolchain";

const path = require("path");
export const command: string = "init [name]";
export const desc: string = "Initialize a PSP project";

const defaultProgramId = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.positional(`name`, {
    type: `string`,
    describe: `the name of your project`,
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  let { name }: any = argv;
  if (!name) {
    console.log(
      "Project name is undefined add a project name with init <project-name>"
    );
    process.exit(0);
  }

  console.log("🚀 Initializing PSP project...");
  const cargoGeneratePath = path.resolve(__dirname, "../../bin/cargo-generate");
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
      "--name",
      name,
      "--define",
      `circom-name=${circomName}`,
      "--define",
      `rust-name=${rustName}`,
      "--define",
      `program-id=${defaultProgramId}`,
    ],
  });
  await executeCommandInDir("yarn", ["install"], name);

  console.log("✅ Project initialized successfully");

  process.exit(0);
};
