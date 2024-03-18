import { spawn, SpawnOptionsWithoutStdio, exec as execCb } from "child_process";
import path from "path";
import { promisify } from "util";
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

const exec = promisify(execCb);
/**
 * Executes a shell command and returns a promise that resolves to the output of the shell command, or an error.
 *
 * @param command A shell command to execute
 * @returns Promise that resolves to string output of shell command
 * @throws {Error} If shell command execution fails
 * @example const output = await execute("ls -alh");
 */
export async function execute(command: string): Promise<string> {
  try {
    const { stdout, stderr } = await exec(command);
    if (!stdout.trim() && stderr.trim()) throw new Error(stderr);
    return stdout;
  } catch (e) {
    console.log("Error in `execute`: ", e);
    return "";
  }
}
