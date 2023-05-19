import { buildPSP } from "../utils/buildPSP";
import type { Arguments, CommandBuilder, Options } from "yargs";

export const command: string = "build";
export const desc: string = "build your PSP";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    name: { type: "string" },
    ptau: { type: "number" },
    circuitDir: { type: "string" },
    // TODO: pass along anchor build options // execsync thingy alt.
  });
//TODO: move all cli-utils to cli ... -> build into bin buildPsP uses macrocircom...

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  let { name, ptau, circuitDir }: any = argv;
  if (!name) {
    console.log(
      "Project name is undefined add a project name with --name <project-name>"
    );
    process.exit(0);
  }

  if (!circuitDir) {
    circuitDir = "circuit";
  }
  if (!ptau) {
    ptau = 15;
  }
  console.log("building PSP...");
  await buildPSP(circuitDir, ptau, name);

  process.exit(0);
};
