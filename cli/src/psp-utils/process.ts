import { spawn, SpawnOptionsWithoutStdio } from "child_process";
import * as path from "node:path";

/**
 * Executes a command and logs the output to the console.
 * @param command - Path to the command to be executed.
 * @param args - Arguments to be passed to the command.
 * @param additionalPath - Additional path to be added to the PATH environment
 * variable.
 */
export async function executeCommand({
  command,
  args,
  additionalPath,
}: {
  command: string;
  args: string[];
  additionalPath?: string;
}): Promise<string> {
  return new Promise((resolve, reject) => {
    const commandBase = path.basename(command);
    let stdoutData = "";

    const childPathEnv = additionalPath
      ? process.env.PATH + path.delimiter + additionalPath
      : process.env.PATH;
    const options: SpawnOptionsWithoutStdio = {
      env: {
        ...process.env,
        PATH: childPathEnv,
      },
    };
    console.log(`Executing command ${commandBase}...`);
    let childProcess;
    try {
      childProcess = spawn(command, args, options);
    } catch (e) {
      throw new Error(`Failed to execute command ${commandBase}: ${e}`);
    }

    childProcess.stdout.on("data", (data: Buffer) => {
      stdoutData += data.toString();
      process.stdout.write(data);
    });

    childProcess.stderr.on("data", (data: Buffer) => {
      process.stderr.write(data);
    });

    childProcess.on("error", (error: Error) => {
      console.log(`${commandBase} failed with error: ${error}`);
      reject(error);
    });

    childProcess.on("close", (code: number) => {
      if (code !== 0) {
        console.log(`${commandBase} exited with code ${code}`);
        reject(new Error(`${commandBase} exited with code ${code}`));
      } else {
        console.log(`${commandBase} finished successfully!`);
        resolve(stdoutData);
      }
    });
  });
}

/**
 * Executes a command in a given directory and logs the output to the console.
 * @param command - Path to the command to be executed.
 * @param args - Arguments to be passed to the command.
 * @param dir - Directory in which the command should be executed.
 * @param options - Options to be passed to the command.
 * @returns {Promise<string>} - The output of the command.
 */
export async function executeCommandInDir(
  command: string,
  args: string[],
  dir: string,
  _options: SpawnOptionsWithoutStdio = {}
): Promise<string> {
  const oldDir = process.cwd();
  process.chdir(dir);
  const result = await executeCommand({
    command,
    args,
  });
  process.chdir(oldDir);
  return result;
}
