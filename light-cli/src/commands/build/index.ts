import { Command, Flags } from "@oclif/core";
import { CustomLoader } from "../../utils/utils";
import { buildPSP } from "../../utils";

class BuildCommand extends Command {
  static description = "Build your PSP";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples: Command.Example[] = [
    "light build --name <project> --ptau 14 --circuitDir ./lib/bin/hello-world ",
  ];

  static flags = {
    name: Flags.string({
      description: "The name of your project",
      required: true,
    }),
    ptau: Flags.integer({
      description: "The value of ptau",
      default: 15,
    }),
    circuitDir: Flags.string({
      description: "The circuit directory",
      default: "circuit",
    }),
  };

  async run() {
    const { flags } = await this.parse(BuildCommand);
    const { name, ptau, circuitDir } = flags;

    const loader = new CustomLoader("Building PSP...");

    loader.start();

    try {
      await buildPSP(circuitDir, ptau, name);
      this.log("\n Built successfully");
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`${error}`);
    }
  }
}

export default BuildCommand;
