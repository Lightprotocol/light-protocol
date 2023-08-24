import { Args, Command, Flags } from "@oclif/core";
import { snakeCase } from "snake-case";
import { executeCargoGenerate } from "../../psp-utils/toolchain";
import * as path from "path";
import { PSP_TEMPLATE_TAG } from "../../psp-utils/constants";
import { camelToScreamingSnake } from "../../utils";
import { toCamelCase } from "../../psp-utils";

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
  static flags = {
    circom: Flags.boolean({
      description:
        "Whether the main circuit is a circom circuit not a .light file.",
      default: false,
      required: false,
    }),
  };

  async run() {
    const { flags, args } = await this.parse(InitCommand);
    let { name } = args;

    this.log("🚀 Initializing PSP project...");

    addCircuit({ name, ...flags });
    this.log("✅ Project initialized successfully");
  }
}

export const addCircuit = async ({
  name,
  circom,
}: {
  name: string;
  circom?: boolean;
}) => {
  var circomName = snakeCase(name);
  var rustName = snakeCase(name);
  let circuit_template = circom
    ? "psp-template/circuits/circuit_circom"
    : "psp-template/circuits/circuit_psp";

  await executeCargoGenerate({
    args: [
      "generate",
      // "--git",
      // "https://github.com/Lightprotocol/psp-template",
      // --tag,
      // PSP_TEMPLATE_TAG,
      "--path",
      "/home/ananas/test_light/psp-template",
      circuit_template,
      "--name",
      name,
      "--define",
      `circom-name=${circomName}`,
      "--define",
      `rust-name=${rustName}`,
      "--define",
      `program-id=${PSP_DEFAULT_PROGRAM_ID}`,
      "--define",
      `VERIFYING_KEY_NAME=${camelToScreamingSnake(circomName)}`,
      "--define",
      `circom-name-camel-case=${toCamelCase(circomName)}`,
      "--vcs",
      "none",
      "--destination",
      `${process.cwd()}/circuits`,
      "--force",
    ],
  });
};
