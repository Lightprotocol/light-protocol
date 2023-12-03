import { executeCargoGenerate } from "./toolchain";
import { PSP_DEFAULT_PROGRAM_ID } from "./constants";
import { kebabCase } from "case-anything";
import {
  camelToScreamingSnake,
  toCamelCase,
  toSnakeCase,
} from "@lightprotocol/zk.js";

export const addCircuit = async ({
  name,
  flags,
}: {
  name: string;
  flags: any;
}) => {
  const circomName = toSnakeCase(name);
  const rustName = toSnakeCase(name);
  const circuit_template = flags.circom
    ? "psp-template/circuits/program_name/circuit_circom"
    : "psp-template/circuits/program_name/circuit_psp";
  const templateSource = flags.path
    ? ["--path", flags.path]
    : [
        ["--git", flags.git],
        flags.tag
          ? ["--tag", flags.tag]
          : flags.branch
          ? ["--branch", flags.branch]
          : ["--branch", "main"],
      ];
  const baseDir = findDirectoryAnchorBaseDirectory(process.cwd());

  await executeCargoGenerate({
    args: [
      "generate",
      ...templateSource.flat(),
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
      `${baseDir}/circuits/${kebabCase(flags.programName)}`,
      "--force",
    ],
  });
};

import * as fs from "fs";
import * as path from "path";

export function findDirectoryAnchorBaseDirectory(startPath: string): string {
  let currentDir = startPath;

  // Check if Anchor.toml exists in the current directory
  while (currentDir !== path.parse(currentDir).root) {
    // Check until we reach the root
    if (fs.existsSync(path.join(currentDir, "Anchor.toml"))) {
      return currentDir;
    }
    currentDir = path.resolve(currentDir, ".."); // Move to the parent directory
  }

  throw new Error(
    `Could not find Anchor.toml in the current directory or any parent directory: ${startPath}`,
  );
}

export function getSubdirectories(baseDir: string): string[] {
  // Check if 'programs' subdirectory exists
  const programsPath = path.join(baseDir, "programs");
  if (
    !fs.existsSync(programsPath) ||
    !fs.statSync(programsPath).isDirectory()
  ) {
    throw new Error(
      `The "programs" directory does not exist in the anchor project: ${baseDir}`,
    );
  }

  // Read the 'programs' directory and filter out non-directory files.
  return fs
    .readdirSync(programsPath, { withFileTypes: true })
    .filter((dirent) => dirent.isDirectory())
    .map((dirent) => dirent.name);
}

export function findAnchorPrograms(): { programs: string[]; baseDir: string } {
  const baseDir = findDirectoryAnchorBaseDirectory(process.cwd());
  return { programs: getSubdirectories(baseDir), baseDir };
}
