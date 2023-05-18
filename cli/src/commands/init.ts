import type { Arguments, CommandBuilder } from "yargs";
import { execSync } from "child_process";
import { Options } from "yargs-parser";
import { anchorBinUrlMap, downloadFileIfNotExists } from "../utils/downloadBin";

const path = require("path");
export const command: string = "init";
export const desc: string = "Initialize a PSP project";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    name: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  let { name }: any = argv;
  if(!name) {
    console.log("Project name is undefined add a project name with --name <project-name>");
    process.exit(0);
  }

  console.log("initing PSP...");
  const anchorPath = path.resolve(__dirname, "../../bin/light-anchor");
  const dirPath = path.resolve(__dirname, "../../bin/");

  await downloadFileIfNotExists(anchorBinUrlMap, anchorPath,dirPath,"light-anchor")

  execSync(`${anchorPath} init-psp ${name}`);

  process.exit(0);
};
