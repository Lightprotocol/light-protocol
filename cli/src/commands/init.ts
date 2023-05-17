import type { Arguments, CommandBuilder } from "yargs";
import { execSync } from "child_process";
import { Options } from "yargs-parser";
const path = require("path");
export const command: string = "init";
export const desc: string = "Initialize a PSP project";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    name: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  let { name }: any = argv;

  console.log("initing PSP...");
  const anchorPath = path.resolve(__dirname, "../../bin/light-anchor");

  execSync(`${anchorPath} init-psp ${name}`);

  process.exit(0);
};
