import * as path from "path";

import {
  downloadCargoGenerateIfNotExists,
  downloadLightBinIfNotExists,
  downloadSolanaIfNotExists,
} from "./download";
import { executeCommand } from "./process";
import { MACRO_CIRCOM_TAG } from "./contants";

const defaultCargoGeneratePath = "../../bin/cargo-generate";
const defaultCircomPath = "../../bin/circom";
const defaultMacroCircomPath = "../../bin/macro-circom";
const defaultAnchorPath = "../../bin/light-anchor";
const defaultSolanaPath = "../../bin/solana";

/**
 * Create a function which looks up for a Light Protocol toolchain binary.
 * The worklow of the function:
 * * Check if the given environment variable is defined. If yes, return its
 *   value. That means that user provided a custom path and we shouldn't manage
 *   it.
 * * If no, check if the binary exists under the default path. If not, download
 *   it. Then return the default path.
 * @param envVarName - The name of the environment variable which contains the
 * path to the binary.
 * @param defaultPath - The default path to the binary.
 * @param downloadFunction - The function which downloads the binary if it
 * doesn't exist.
 * @param downloadParams - The parameters for the download function.
 * @returns {Function}
 */
function createPathFunction({
  envVarName,
  defaultPath,
  downloadFunction,
  downloadParams = {},
}: {
  envVarName: string;
  defaultPath: string;
  downloadFunction: Function;
  downloadParams?: object;
}) {
  return async function (): Promise<string> {
    const envPath = process.env[envVarName];
    if (envPath) {
      return envPath;
    }

    const localFilePath = path.resolve(__dirname, defaultPath);
    const dirPath = path.resolve(__dirname, "../../bin");
    await downloadFunction({
      localFilePath,
      dirPath,
      ...downloadParams,
    });
    return localFilePath;
  };
}

/**
 * Look up for the path to the cargo-generate binary.
 * @returns {Promise<string>}
 */
const cargoGeneratePath = createPathFunction({
  envVarName: "LIGHT_PROTOCOL_CARGO_GENERATE_PATH",
  defaultPath: defaultCargoGeneratePath,
  downloadFunction: downloadCargoGenerateIfNotExists,
});

/**
 * Look up for the path to the circom binary.
 * @returns {Promise<string>}
 */
const circomPath = createPathFunction({
  envVarName: "LIGHT_PROTOCOL_CIRCOM_PATH",
  defaultPath: defaultCircomPath,
  downloadFunction: downloadLightBinIfNotExists,
  downloadParams: { repoName: "circom", remoteFileName: "circom" },
});

/**
 * Look up for the path to the macro-circom binary.
 * @returns {Promise<string>}
 */
const macroCircomPath = createPathFunction({
  envVarName: "LIGHT_PROTOCOL_MACRO_CIRCOM_PATH",
  defaultPath: defaultMacroCircomPath,
  downloadFunction: downloadLightBinIfNotExists,
  downloadParams: { repoName: "macro-circom", remoteFileName: "macro-circom", tag: MACRO_CIRCOM_TAG},
});

/**
 * Look up for the path to the anchor binary.
 * @returns {Promise<string>}
 */
const anchorPath = createPathFunction({
  envVarName: "LIGHT_PROTOCOL_ANCHOR_PATH",
  defaultPath: defaultAnchorPath,
  downloadFunction: downloadLightBinIfNotExists,
  downloadParams: { repoName: "anchor", remoteFileName: "light-anchor" },
});

/**
 * Look up for the path to the Solana toolchain.
 * @returns {Promise<string>}
 */
async function solanaPath(): Promise<string> {
  const envPath = process.env["LIGHT_PROTOCOL_SOLANA_PATH"];
  if (envPath) {
    return Promise.resolve(envPath);
  }

  const dirPath = path.resolve(__dirname, defaultSolanaPath);
  await downloadSolanaIfNotExists({
    dirPath,
  });
  return dirPath;
}

/**
 * Create a function which executes a binary with the given arguments.
 * @param pathFunction - The function which looks up for the path to the binary.
 * @returns {Function}
 */
function createExecuteFunction(pathFunction: Function) {
  return async function ({ args }: { args: string[] }): Promise<string> {
    const command = await pathFunction();
    return await executeCommand({
      command,
      args,
    });
  };
}

/**
 * Execute the cargo-generate binary with the given arguments.
 * @param args - The arguments for the cargo-generate binary.
 * @returns {Promise<string>}
 */
export const executeCargoGenerate = createExecuteFunction(cargoGeneratePath);

/**
 * Execute the circom binary with the given arguments.
 * @param args - The arguments for the circom binary.
 * @returns {Promise<string>}
 */
export const executeCircom = createExecuteFunction(circomPath);

/**
 * Execute the macro-circom binary with the given arguments.
 * @param args - The arguments for the macro-circom binary.
 * @returns {Promise<string>}
 */
export const executeMacroCircom = createExecuteFunction(macroCircomPath);

/**
 * Execute the anchor binary with the given arguments.
 * @param args - The arguments for the anchor binary.
 * @returns {Promise<string>}
 */
export async function executeAnchor({
  args,
}: {
  args: string[];
}): Promise<string> {
  const command = await anchorPath();
  return await executeCommand({
    command,
    args,
  });
}
