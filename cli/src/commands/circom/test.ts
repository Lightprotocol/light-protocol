import { Args, Command } from "@oclif/core";
import { toSnakeCase } from "../../psp-utils/utils";
import { start_test_validator } from "../../utils";
import { executeCommand } from "../../psp-utils";
import { PSP_DEFAULT_PROGRAM_ID } from "./init";

export default class TestCommand extends Command {
  static description =
    "Deploys your circom-anchor on a local testnet and runs test";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the circom-anchor project.",
      required: true,
    }),
    address: Args.string({
      name: "ADDRESS",
      description: "The address of the Anchor program.",
      required: false,
      default: PSP_DEFAULT_PROGRAM_ID,
    }),
  };

  async run() {
    const { args } = await this.parse(TestCommand);
    let { name, address } = args;

    const programName = toSnakeCase(name!);
    const path = `./target/deploy/${programName}.so`;
    await start_test_validator({
      additonalPrograms: [{ address: address, path }],
    });

    await executeCommand({
      command: `yarn`,
      args: [`ts-mocha`, `-t`, `2000000`, `tests/${name}.ts`, `--exit`],
    });
    this.exit(0);
  }
}
