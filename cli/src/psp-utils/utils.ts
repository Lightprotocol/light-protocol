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
export function findFile({
  directory,
  extension,
}: {
  directory: string;
  extension: string;
}): string {
  const files = fs.readdirSync(directory);
  const lightFiles = files.filter((file) => file.endsWith(`.${extension}`));

  if (lightFiles.length > 1) {
    throw new Error(`More than one .${extension} file found in the directory.`);
  } else if (lightFiles.length === 1) {
    return lightFiles[0];
  } else {
    throw new Error(`No .${extension} files found in the directory.`);
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
