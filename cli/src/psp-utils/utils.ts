import * as fs from "fs";

/**
 * Extracts the circuit filename from the input string using a regex pattern.
 * @param input - The string to extract the circuit filename from. Should be the stdout of the circom macro.
 * Sample input: "sucessfully created main tmpTestPspMain.circom and tmp_test_psp.circom"
 */
export function extractFilename(input: string): string | null {
  const regex = /main\s+(\S+\.circom)/;
  const match = input.match(regex);
  return match ? match[1] : null;
}

/**
 * Searches for a file with the @param extension in a specified directory.
 * Throws an error if more than one such file or no such file is found.
 * @param directory - The directory to search for the .light file.
 * @returns {string} - The name of the .light file found in the directory.
 */
// export function findFile({
//   directory,
//   extension,
// }: {
//   directory: string;
//   extension: string;
// }): string {
//   const files = fs.readdirSync(directory);
//   const lightFiles = files.filter((file) => file.endsWith(`.${extension}`));

//   if (lightFiles.length > 1) {
//     throw new Error(`More than one .${extension} file found in the directory.`);
//   } else if (lightFiles.length === 1) {
//     return lightFiles[0];
//   } else {
//     throw new Error(`No .${extension} files found in the directory.`);
//   }
// }

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
}): { filename: string; fullPath: string } {
  return recursiveSearch(directory, extension);
}

function recursiveSearch(
  directory: string,
  extension: string
): { filename: string; fullPath: string } {
  const entries = fs.readdirSync(directory);
  const matchingFiles: { filename: string; fullPath: string }[] = [];

  for (const entry of entries) {
    const fullPath = path.join(directory, entry);
    const stats = fs.statSync(fullPath);

    if (stats.isDirectory()) {
      try {
        const fileFound = recursiveSearch(fullPath, extension);
        if (fileFound) matchingFiles.push(fileFound);
      } catch (e) {
        // You can either handle the error here or propagate it, depending on your use case
      }
    } else if (stats.isFile() && entry.endsWith(`${extension}`)) {
      matchingFiles.push({ filename: entry, fullPath });
    }
  }

  if (matchingFiles.length > 1) {
    throw new Error(`More than one .${extension} file found.`);
  } else if (matchingFiles.length === 1) {
    return matchingFiles[0];
  } else {
    throw new Error(`No .${extension} files found.`);
  }
}

export function toSnakeCase(str: string): string {
  return str.replace(/-/g, "_");
}

export const snakeCaseToCamelCase = (
  str: string,
  uppercaseFirstLetter: boolean = false
) =>
  str
    .split("_")
    .reduce(
      (res, word, i) =>
        i === 0 && !uppercaseFirstLetter
          ? word.toLowerCase()
          : `${res}${word.charAt(0).toUpperCase()}${word
              .substr(1)
              .toLowerCase()}`,
      ""
    );
import { resolve as resolvePath } from "path";

export async function renameFolder(
  oldPath: string,
  newPath: string
): Promise<void> {
  fs.rename(resolvePath(oldPath), resolvePath(newPath), (err) => {
    if (err) {
      throw err;
    }
  });
}
