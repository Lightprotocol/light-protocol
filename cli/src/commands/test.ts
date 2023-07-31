import { toSnakeCase } from "../utils/buildPSP";
import type { Arguments, CommandBuilder, Options } from "yargs";
import path = require("path");
import * as fs from "fs";
import { executeCommand } from "../utils/process";
import { sleep } from "@lightprotocol/zk.js";
import { downloadBinIfNotExists } from "../utils/download";

export const command: string = "test";
export const desc: string = "Deploys your PSP on a local testnet and runs test";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    // network: { type: "string" },
    projectName: { type: "string" },
    programAddress: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  let { projectName, programAddress }: any = argv;
  if (!projectName) {
    console.log(
      "Project name is undefined. Add a project name with --projectName <project-name>"
    );
    process.exit(0);
  }
  if (!programAddress) {
    console.log(
      "Program address is undefined. Add a program address with --programAddress <program-address>"
    );
    process.exit(0);
  }
  const programName = toSnakeCase(projectName);

  await start_test_validator(projectName, programName, programAddress);
  process.exit(0);
};

export async function start_test_validator(
  projectName: string,
  programName: string,
  programAddress: string
) {
  const command = "solana-test-validator";
  const LIMIT_LEDGER_SIZE = "500000000";
  const BASE_PATH = "/bin/";
  const programs = [
    { id: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV", name: "spl_noop.so" },
    {
      id: "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6",
      name: "merkle_tree_program.so",
    },
    {
      id: "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i",
      name: "verifier_program_zero.so",
    },
    {
      id: "DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj",
      name: "verifier_program_storage.so",
    },
    {
      id: "J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc",
      name: "verifier_program_one.so",
    },
    {
      id: "2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86",
      name: "verifier_program_two.so",
    },
    { id: programAddress, path: `./target/deploy/${programName}.so` },
  ];

  const dirPath = path.resolve(__dirname, BASE_PATH);

  let solanaArgs = [
    "--reset",
    `--limit-ledger-size=${LIMIT_LEDGER_SIZE}`,
    "--quiet",
  ];

  for (let program of programs) {
    let dirPathString = "../../bin/" + program.name;
    const localFilePath = path.resolve(__dirname, dirPathString);
    if (!program.path) {
      await downloadBinIfNotExists({
        localFilePath,
        dirPath,
        owner: "Lightprotocol",
        repoName: "light-protocol",
        remoteFileName: program.name,
      });
    }

    let path1 = program.path ? program.path : `${localFilePath}`;
    solanaArgs.push("--bpf-program", program.id, path1);
  }
  let dirPathString = "../../accounts/";
  const localFilePath = path.resolve(__dirname, dirPathString);
  console.log("accounts path ", localFilePath);
  solanaArgs.push("--account-dir", localFilePath);

  try {
    // killall process
    await executeCommand({
      command: "killall",
      args: ["solana-test-validator"],
    });
  } catch (error) {}
  try {
    // killall process
    await executeCommand({
      command: "killall",
      args: ["solana-test-val"],
    });
  } catch (error) {}

  await new Promise((r) => setTimeout(r, 1000));

  executeCommand({
    command,
    args: [...solanaArgs],
  });
  await sleep(10000);
  await executeCommand({
    command: `yarn`,
    args: [`ts-mocha`, `-t`, `2000000`, `tests/${projectName}.ts`, `--exit`],
  });
}

// @ananas-block: currently not used should be implemented for robustness
// Function that climbs each parent directory from a given starting directory until it finds a package.json
export async function discoverFromPath(
  startFrom: string
): Promise<string | null> {
  let currentPath: string | null = startFrom;

  while (currentPath) {
    try {
      const files = fs.readdirSync(currentPath);

      for (const file of files) {
        const filePath = path.join(currentPath, file);

        if (file === "package.json") {
          return filePath;
        }
      }

      // Not found. Go up a directory level.
      const parentPath = path.dirname(currentPath);
      if (parentPath === currentPath) {
        currentPath = null;
      } else {
        currentPath = parentPath;
      }
    } catch (err) {
      console.error(
        `Error reading the directory with path: ${currentPath}`,
        err
      );
      currentPath = null;
    }
  }

  return null;
}
