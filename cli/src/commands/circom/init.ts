import { Command, Args } from "@oclif/core";
import { ProjectType, initRepo } from "../psp/init";

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

    await initRepo(name, ProjectType.CIRCOM);
    // const rustName = snakeCase(name);
    // const circomName = snakeCaseToCamelCase(rustName);
    // const programName = snakeCaseToCamelCase(rustName, true);

    // await executeCargoGenerate({
    //   args: [
    //     "generate",
    //     // "--git",
    //     // "https://github.com/Lightprotocol/circom-anchor-template.git",
    //     "--path",
    //     "/home/ananas/test_light/psp-template",
    //     "circom-anchor-template",
    //     "--name",
    //     name,
    //     "--define",
    //     `circom-name=${toSnakeCase(circomName)}`,
    //     "--define",
    //     `rust-name=${rustName}`,
    //     "--define",
    //     `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
    //     "--define",
    //     `anchor-program-name=${programName}`,
    //     "--define",
    //     `circom-name-camel-case=${circomName}`,
    //     "--define",
    //     `VERIFYING_KEY_NAME=${camelToScreamingSnake(circomName)}`,
    //   ],
    // });
    // await renameFolder(`${process.cwd()}/${name}/circuits/circuit`, `${process.cwd()}/${name}/circuits/${name}`);
    // await removeFile(`${process.cwd()}/${name}/circuits/cargo-generate.toml`);
    // this.log("Executing yarn install in dir ", name);
    // await executeCommandInDir("yarn", ["install"], name);

    this.log("✅ Project initialized successfully");
  }
}
