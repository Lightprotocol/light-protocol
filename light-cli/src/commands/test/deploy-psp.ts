import { Command, Flags } from "@oclif/core";
import { execSync } from "child_process";
import { toSnakeCase } from "@lightprotocol/zk.js";
const path = require("path");

class TestCommand extends Command {
  static description = "Deploys your PSP on a local testnet and runs tests";

  static flags = {
    projectName: Flags.string({
      description: "The name of your project",
      required: true,
    }),
    programAddress: Flags.string({
      description: "The program address",
      required: true,
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { flags } = await this.parse(TestCommand);
    const { projectName, programAddress } = flags;

    const programName = toSnakeCase(projectName);
    const commandPath = path.resolve(__dirname, "../../scripts/runTest.sh");
    const systemProgramPath = path.resolve(__dirname, "../../");

    try {
      const stdout = execSync(
        `${commandPath} ${systemProgramPath} ${process.cwd()} ${programAddress} ${programName}.so 'yarn ts-mocha -t 2000000 tests/${projectName}.ts --exit'`
      );
      this.log(stdout.toString().trim());
    } catch (err) {
      this.error(`${err}`);
    }
  }
}

export default TestCommand;
