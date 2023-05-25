import { spawn, SpawnOptionsWithoutStdio } from "child_process";
import path from "path";

/**
 * Executes a command and logs the output to the console.
 * @param command - Path to the command to be executed.
 * @param args - Arguments to be passed to the command.
 * @param options - Options to be passed to the command.
 */
export async function executeCommand(
  command: string,
  args: string[],
  options: SpawnOptionsWithoutStdio = {}
): Promise<string> {
  return new Promise((resolve, reject) => {
    let commandBase = path.basename(command);
    let stdoutData = "";

    let childProcess;
    try {
      childProcess = spawn(command, args, options);
    } catch (e) {
      throw new Error(`Failed to execute command ${commandBase}: ${e.message}`);
    }

    childProcess.stdout.on("data", (data: Buffer) => {
      stdoutData += data.toString();
      process.stdout.write(data);
    });

    childProcess.stderr.on("data", (data: Buffer) => {
      process.stderr.write(data);
    });

    childProcess.on("error", (error: Error) => {
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
