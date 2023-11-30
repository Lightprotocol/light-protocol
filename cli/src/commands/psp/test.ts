import { Args, Command, Flags } from "@oclif/core";
import { sleep, toSnakeCase } from "@lightprotocol/zk.js";
import { startTestValidator } from "../../utils";
import { executeCommand, PSP_DEFAULT_PROGRAM_ID } from "../../psp-utils";
import { findAnchorPrograms } from "../../psp-utils/addCircuit";

export default class TestCommand extends Command {
  static description = "Deploys your PSP on a local testnet and runs test";

  static args = {
    name: Args.string({
      name: "TEST_NAME",
      description: "The name of the test located in tests/${name}.ts",
      required: true,
    }),
    // address: Args.string({
    //   name: "ADDRESS",
    //   description: "The address of the PSP.",
    //   required: false,
    //   default: PSP_DEFAULT_PROGRAM_ID,
    // }),
  };

  static flags = {
    time: Flags.string({
      char: "t",
      description:
        "Wait time for test validator to start, default is 15s (15000).",
      default: "15000",
      required: false,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(TestCommand);
    const { name } = args;

    const { baseDir, programs } = findAnchorPrograms();
    const additionalPrograms = parseIdlFiles(baseDir + "/target/idl");
    if (additionalPrograms.length === 0) {
      throw new Error("No programs found");
    }
    startTestValidator({
      additionalPrograms,
    });
    await sleep(Number(flags.time));

    await executeCommand({
      command: `pnpm`,
      args: [`ts-mocha`, `-t`, `2000000`, `tests/${name}.ts`, `--exit`],
    });
    this.exit(0);
  }
}
import * as fs from "fs";
import * as path from "path";
export type AdditionalProgram = {
  address: string;
  path: string;
};
function parseIdlFiles(dir: string): AdditionalProgram[] {
  const files = fs.readdirSync(dir);

  const programs: AdditionalProgram[] = [];

  for (const file of files) {
    const fullPath = path.join(dir, file);
    if (fullPath.endsWith(".json")) {
      const jsonData = JSON.parse(fs.readFileSync(fullPath, "utf-8"));
      const programId = jsonData.constants?.find(
        (constant: any) => constant.name === "PROGRAM_ID"
      )?.value;

      if (programId) {
        programs.push({
          address: programId.split('"')[1],
          path: path.join(
            dir.split("idl")[0],
            `deploy/${file.split(".")[0]}.so`
          ),
        });
      } else {
        throw new Error(`PROGRAM_ID not found in ${file}`);
      }
    }
  }
  return programs;
}
