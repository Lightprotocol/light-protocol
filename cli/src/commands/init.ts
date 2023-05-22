import type { Arguments, CommandBuilder } from "yargs";
import { execSync } from "child_process";
import { Options } from "yargs-parser";
import { snakeCase } from "snake-case";
import { downloadCargoGenerateIfNotExists } from "../utils/downloadBin";

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

  console.log("initing PSP...");
  const cargoGeneratePath = path.resolve(__dirname, "../../bin/cargo-generate");
  const dirPath = path.resolve(__dirname, "../../bin/");

  await downloadCargoGenerateIfNotExists({
    localFilePath: cargoGeneratePath,
    dirPath,
  });

  const circomName = snakeCase(name);
  const rustName = snakeCase(name);
  execSync(
    `${cargoGeneratePath} generate \
    --git https://github.com/Lightprotocol/psp-template \
    --branch vadorovsky/cargo-generate-template \
    --name ${name} \
    --define circom-name=${circomName} \
    --define rust-name=${rustName} \
    --define program-id=${defaultProgramId}`
  );

  console.log("Project initialized successfully");

  process.exit(0);
};
