import { sleep } from "@lightprotocol/zk.js";
import * as fs from "fs";

/**
 * Extracts the circuit filename from the input string using a regex pattern.
 * @param input - The string to extract the circuit filename from. Should be the stdout of the circom macro.
 * Sample input: "sucessfully created main tmpTestPspMain.circom and tmp_test_psp.circom"
 */
export function extractFilename(input: string): string | null {
  const regex = /main\s+(\S+\.circom)/;
  const match = input.match(regex);
  console.log("input ");
  console.log("input ", input);
  console.log("match ", match);
  return match ? match[1] : null;
}

import path from "path";

/**
 * Recursively searches for a file with the @param extension in a specified directory.
 * Throws an error if more than one such file or no such file is found.
 * @param directory - The directory to search for the file.
 * @returns { filename: string, fullPath: string } - The name of the file and its full path found in the directory or its subdirectories.
 */
export function findFile({
  directory,
  extension,
}: {
  directory: string;
  extension: string;
}): { filename: string; fullPath: string; light?: boolean }[] {
  return recursiveSearch(directory, extension);
}

function recursiveSearch(
  directory: string,
  extension: string,
): { filename: string; fullPath: string; light?: boolean }[] {
  const entries = fs.readdirSync(directory);
  const matchingFiles: {
    filename: string;
    fullPath: string;
    light?: boolean;
  }[] = [];
  let lightFilesCount = 0;
  for (const entry of entries) {
    const fullPath = path.join(directory, entry);
    const stats = fs.statSync(fullPath);

    if (stats.isDirectory()) {
      try {
        const fileFound = recursiveSearch(fullPath, extension);
        if (fileFound) fileFound.map((value) => matchingFiles.push(value));
      } catch (e) {
        // You can either handle the error here or propagate it, depending on your use case
      }
    } else if (stats.isFile() && entry.endsWith(".light")) {
      lightFilesCount++;
    }
    if (stats.isFile() && entry.endsWith(`${extension}`)) {
      matchingFiles.push({ filename: entry, fullPath });
    }
  }
  if (lightFilesCount > 1)
    matchingFiles.map((value) => {
      return { ...value, light: true };
    });
  return matchingFiles;
}

import { resolve as resolvePath } from "path";

export function renameFolder(oldPath: string, newPath: string) {
  fs.renameSync(resolvePath(oldPath), resolvePath(newPath));
}
