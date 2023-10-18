import { Args, Command, Flags } from "@oclif/core";
import { sleep, toSnakeCase } from "@lightprotocol/zk.js";
import { startTestValidator } from "../../utils";
import { executeCommand, PSP_DEFAULT_PROGRAM_ID } from "../../psp-utils";

export default class TestCommand extends Command {
  static description = "Deploys your PSP on a local testnet and runs test";

  static args = {
    name: Args.string({
      name: "NAME",
      description: "The name of the PSP project.",
      required: true,
    }),
    address: Args.string({
      name: "ADDRESS",
      description: "The address of the PSP.",
      required: false,
      default: PSP_DEFAULT_PROGRAM_ID,
    }),
  };

  static flags = {
    time: Flags.string({
      char: "t",
      description: "Wait time for test validator to start, default is 15s.",
      default: "15000",
      required: false,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TestCommand);
    const { name, address } = args;

    const programName = toSnakeCase(name!);
    const path = `./target/deploy/${programName}.so`;
    startTestValidator({
      additionalPrograms: [{ address: address!, path }],
    });
    await sleep(Number(flags.time));

    await executeCommand({
      command: `pnpm`,
      args: [`ts-mocha`, `-t`, `2000000`, `tests/${name}.ts`, `--exit`],
    });
    this.exit(0);
  }
}
