import { spawn, SpawnOptionsWithoutStdio } from "child_process";
import path from "path";
import fs from "fs";
import find from "find-process";
import { exec as execCb } from "node:child_process";
import { promisify } from "util";

export async function killProcessByName(processName: string) {
  const processList = await find("name", processName);
  for (const proc of processList) {
    process.kill(proc.pid);
  }
}

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
  logFile = true,
}: {
  command: string;
  args: string[];
  additionalPath?: string;
  logFile?: boolean;
}): Promise<string> {
  return new Promise((resolve, reject) => {
    const commandBase = path.basename(command);
    let stdoutData = "";

    const childPathEnv = additionalPath
      ? process.env.PATH + path.delimiter + additionalPath
      : process.env.PATH;

    const options: SpawnOptionsWithoutStdio = {
      env: childPathEnv ? { ...process.env, PATH: childPathEnv } : process.env,
      detached: true,
    };

    if (logFile) {
      const folderName = "test-ledger";
      const file = `./${folderName}/${commandBase}.log`;

      if (!fs.existsSync(folderName)) {
        fs.mkdirSync(folderName);
      }

      const logStream: fs.WriteStream = fs.createWriteStream(file, {
        flags: "a",
      });
      process.stdout.pipe(logStream);
      process.stderr.pipe(logStream);
    }

    console.log(`Executing command ${commandBase} ${args}...`);
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
    const { stdout } = await exec(command);
    return stdout;
  } catch (err) {
    console.log("Error in `execute`: ", err);
    throw err;
  }
}

export function spawnBinaryByName(binaryName: string, args: string[] = []) {
  const binDir = path.join(__dirname, "../..", "bin");
  const command = path.join(binDir, binaryName);

  const logDir = path.join(__dirname, "test-ledger");
  if (!fs.existsSync(logDir)) {
    fs.mkdirSync(logDir);
  }

  const out = fs.openSync(`${logDir}/${binaryName}.log`, "a");
  const err = fs.openSync(`${logDir}/${binaryName}.log`, "a");

  const spawnedProcess = spawn(command, args, {
    stdio: ["ignore", out, err],
    shell: false,
    detached: true,
  });

  spawnedProcess.on("close", (code) => {
    console.log(`${binaryName} process exited with code ${code}`);
  });
}

export function spawnBinary(
  binaryName: string,
  cli_bin: boolean,
  args: string[] = [],
) {
  let command = binaryName;
  if (cli_bin) {
    const binDir = path.join(__dirname, "../..", "bin");
    command = path.join(binDir, binaryName);
  }

  if (!fs.existsSync("test-ledger")) {
    fs.mkdirSync("test-ledger");
  }

  const out = fs.openSync(`test-ledger/${binaryName}.log`, "a");
  const err = fs.openSync(`test-ledger/${binaryName}.log`, "a");

  const spawnedProcess = spawn(command, args, {
    stdio: ["ignore", out, err],
    shell: false,
  });

  spawnedProcess.on("error", (error) => {
    console.error(`error: ${error.message}`);
  });

  spawnedProcess.on("close", (code) => {
    console.log(`${binaryName} process exited with code ${code}`);
  });
}
