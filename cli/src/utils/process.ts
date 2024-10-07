import { spawn, SpawnOptionsWithoutStdio } from "child_process";
import path from "path";
import fs from "fs";
import find from "find-process";
import { exec as execCb } from "node:child_process";
import { promisify } from "util";
import axios from "axios";
const waitOn = require("wait-on");

export async function killProcess(processName: string) {
  const processList = await find("name", processName);
  const targetProcesses = processList.filter(
    (proc) => proc.cmd.split(" ")[0] === processName,
  );
  targetProcesses.forEach((proc) => {
    process.kill(proc.pid);
  });
}

export async function killProcessByPort(port: number) {
  await execute(`lsof -t -i:${port} | while read line; do kill -9 $line; done`);
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
  env,
}: {
  command: string;
  args: string[];
  additionalPath?: string;
  logFile?: boolean;
  env?: NodeJS.ProcessEnv;
}): Promise<string> {
  return new Promise((resolve, reject) => {
    const commandParts = command.split(" && ");
    const finalCommand = commandParts.pop() || "";
    const preCommands = commandParts.join(" && ");

    const fullCommand = preCommands
      ? `${preCommands} && ${finalCommand} ${args.join(" ")}`
      : `${finalCommand} ${args.join(" ")}`;

    const commandBase = path.basename(finalCommand);
    let stdoutData = "";

    const childPathEnv = additionalPath
      ? process.env.PATH + path.delimiter + additionalPath
      : process.env.PATH;

    const options: SpawnOptionsWithoutStdio = {
      env:
        env ||
        (childPathEnv ? { ...process.env, PATH: childPathEnv } : process.env),
      shell: true,
      detached: true,
    };

    let logStream: fs.WriteStream | null = null;

    if (logFile) {
      const folderName = "test-ledger";
      const file = `./${folderName}/${commandBase}.log`;

      if (!fs.existsSync(folderName)) {
        fs.mkdirSync(folderName);
      }

      logStream = fs.createWriteStream(file, { flags: "a" });
    }

    console.log(`Executing command: ${fullCommand}`);
    let childProcess;
    try {
      childProcess = spawn(fullCommand, [], options);
    } catch (e) {
      throw new Error(`Failed to execute command ${commandBase}: ${e}`);
    }

    if (logStream) {
      childProcess.stdout.pipe(logStream);
      childProcess.stderr.pipe(logStream);
    }

    childProcess.stdout.on("data", (data: Buffer) => {
      stdoutData += data.toString();
      process.stdout.write(data);
    });

    childProcess.stderr.on("data", (data: Buffer) => {
      process.stderr.write(data);
    });

    childProcess.on("close", (code: number) => {
      if (logStream) {
        logStream.end();
      }
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

export function spawnBinary(command: string, args: string[] = []) {
  const logDir = "test-ledger";
  const binaryName = path.basename(command);

  const dir = path.join(__dirname, "../..", logDir);
  try {
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    const logPath = path.join(dir, `${binaryName}.log`);
    const out = fs.openSync(logPath, "a");
    const err = fs.openSync(logPath, "a");

    const spawnedProcess = spawn(command, args, {
      stdio: ["ignore", out, err],
      shell: false,
      detached: true,
    });

    spawnedProcess.on("close", (code) => {
      console.log(`${binaryName} process exited with code ${code}`);
    });

    return spawnedProcess;
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(`Error spawning binary: ${error.message}`);
    } else {
      console.error(`An unknown error occurred while spawning binary`);
    }
    throw error;
  }
}

export async function waitForServers(
  servers: { port: number; path: string }[],
) {
  const opts = {
    resources: servers.map(
      ({ port, path }) => `http-get://127.0.0.1:${port}${path}`,
    ),
    delay: 1000,
    timeout: 25000,
    interval: 300,
    simultaneous: 2,
    validateStatus: function (status: number) {
      return (
        (status >= 200 && status < 300) || status === 404 || status === 405
      );
    },
  };

  try {
    await waitOn(opts);
    servers.forEach((server) => {
      console.log(`${server.port} is up!`);
    });
  } catch (err) {
    console.error("Error waiting for server to start:", err);
    throw err;
  }
}

// Solana test validator can be unreliable when starting up.
export async function confirmServerStability(
  url: string,
  attempts: number = 20,
) {
  try {
    for (let i = 0; i < attempts; i++) {
      const response = await axios.get(url);
      if (response.status !== 200) {
        throw new Error("Server failed stability check");
      }
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    console.log("Server has passed stability checks.");
  } catch (error) {
    console.error("Server stability check failed:", error);
    throw error;
  }
}
