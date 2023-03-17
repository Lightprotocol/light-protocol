import { exec, ExecException } from "child_process";
import concat from "concat-stream";

export interface Options {
  timeout?: number;
  maxTimeout?: number;
  env?: {
    DEBUG: boolean;
  } | null;
}

export const KEYS = {
  ENTER: "\x0D",
  DOWN: "\x1B\x5B\x42",
  UP: "\x1B\x5B\x41",
  SPACE: "\x20",
  ESCAP: "\x1B",
};

export const  runCommand = (
  command: string,
  callback: {
    (): void;
    (arg0: ExecException | null, arg1: string, arg2: string): void;
  }
) => {
  return exec(
    command,
    (function () {
      return function (error, data, stderr) {
        if (!callback) return;

        callback(error, data, stderr);
      };
    })()
  );
};

/**
 * Creates a command and executes inputs (user responses) to the stdin
 * Returns a promise that resolves when all inputs are sent
 * Rejects the promise if any error
 * @param {string} command command to the process to execute
 * @param {Array} inputs (Optional) Array of inputs (user responses)
 * @param {Object} opts (optional) Environment variables
 */

export const executeWithInput = (
  command: string,
  inputs: string[] = [],
  opts: Options = { env: null, timeout: 200, maxTimeout: 100000 }
): Promise<string> => {
  if (!Array.isArray(inputs)) {
    opts = inputs;
    inputs = [];
  }
  const { env, timeout, maxTimeout } = opts;
  const childProcess: any = runCommand(command, () => { });

  childProcess.stdin!.setEncoding("utf-8");

  let currentInputTimeout: string | number | NodeJS.Timeout | undefined;
  let killIOTimeout: string | number | NodeJS.Timeout | undefined;

  const loop = (statments: string[] | any[]) => {
    if (killIOTimeout) {
      clearTimeout(killIOTimeout);
    }

    if (!statments.length) {
      childProcess.stdin!.end();

      // Set a timeout to wait for CLI response. If CLI takes longer than
      // maxTimeout to respond, kill the childProcess and notify user
      killIOTimeout = setTimeout(() => {
        console.error("Error: Reached I/O timeout");
        // @ts-ignore
        childProcess.kill(constants.signals.SIGTERM);
      }, maxTimeout);

      return;
    }

    currentInputTimeout = setTimeout(() => {
      childProcess.stdin!.write(statments[0]);
      // Log debug I/O statements on tests
      if (env && env.DEBUG) {
        console.log(`input: ${statments[0]}`, "info");
      }
      loop(statments.slice(1));
    }, timeout);
  };

  const promise = new Promise((resolve, reject) => {
    // Get errors from CLI
    childProcess.stderr!.on("data", (data: any) => {
      // Log debug I/O statements on tests
      if (env && env.DEBUG) {
        console.log(`error: ${data.toString()}`, "error");
      }
    });

    // Get output from CLI
    childProcess.stdout!.on("data", (data: any) => {
      // Log debug I/O statements on tests
      if (env && env.DEBUG) {
        console.log(`output: ${data.toString()}`, "info");
      }
    });

    // childProcess.stderr!.once("data", (error: any) => {
    //   childProcess.stdin!.end();

    //   if (currentInputTimeout) {
    //     clearTimeout(currentInputTimeout);
    //     inputs = [];
    //   }
    //   console.log("Error here", error)
    //   reject(error.toString());
    // });

    // childProcess.on("error", reject);

    // Kick off the process
    loop(inputs);
    childProcess.stdout!.pipe(
      concat((result) => {
        if (killIOTimeout) {
          clearTimeout(killIOTimeout);
        }
        resolve(result.toString());
      })
    );
  });

  // Appending the process to the promise, in order to
  // add additional parameters or behavior (such as IPC communication)
  // @ts-ignore

  promise.attachedProcess = childProcess;
  // @ts-ignore

  return promise;
};