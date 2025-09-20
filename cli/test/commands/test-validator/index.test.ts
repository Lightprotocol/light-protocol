import { expect } from "chai";
import { defaultSolanaWalletKeypair, programsDirPath } from "../../../src";
import { Connection, Keypair } from "@solana/web3.js";
import * as path from "path";
import * as fs from "fs";
import { killProcess } from "../../../src/utils/process";
import { exec as execCb } from "child_process";
import { promisify } from "util";
import { ExecException } from "node:child_process";

const exec = promisify(execCb);

describe("test-validator command", function () {
  this.timeout(120_000);

  const defaultRpcPort = 8899;
  const customRpcPort = 8877;
  const defaultIndexerPort = 8784;
  const customIndexerPort = 8755;
  const defaultProverPort = 3001;
  const customProverPort = 3002;

  async function cleanupProcesses() {
    console.log("Running cleanup...");
    try {
      try {
        await exec("./test_bin/dev test-validator --stop");
        console.log("Validator stopped via CLI command");
      } catch (e) {
        console.log("No running validator to stop via CLI");
      }

      await killProcess("solana-test-validator");
      await killProcess("photon");
      await killProcess("prover");

      await new Promise((resolve) => setTimeout(resolve, 8000));

      await verifyProcessStopped(defaultRpcPort, "Validator");
      await verifyProcessStopped(defaultIndexerPort, "Indexer");
      await verifyProcessStopped(defaultProverPort, "Prover");

      console.log("Cleanup completed successfully");
    } catch (error) {
      console.error("Error in cleanup:", error);
      throw error;
    }
  }

  async function verifyProcessStopped(
    port: number,
    serviceName: string,
    maxRetries: number = 3,
  ) {
    for (let i = 0; i < maxRetries; i++) {
      try {
        await fetch(`http://localhost:${port}/health`, {
          signal: AbortSignal.timeout(2000),
        });
        if (i === maxRetries - 1) {
          throw new Error(`${serviceName} still running after cleanup`);
        }
        console.log(`${serviceName} still running, waiting...`);
        await new Promise((resolve) => setTimeout(resolve, 2000));
      } catch (e) {
        console.log(`${serviceName} stopped`);
        return;
      }
    }
  }

  async function waitForValidatorReady(
    port: number = defaultRpcPort,
    maxRetries: number = 15,
    delayMs: number = 3000,
  ): Promise<Connection> {
    for (let i = 0; i < maxRetries; i++) {
      try {
        console.log(
          `Attempting to connect to validator (attempt ${i + 1}/${maxRetries})...`,
        );
        const connection = new Connection(`http://localhost:${port}`, {
          commitment: "confirmed",
          confirmTransactionInitialTimeout: 5000,
        });

        const version = await Promise.race([
          connection.getVersion(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("Timeout")), 5000),
          ),
        ]);

        console.log("Validator is ready and responding");
        return connection;
      } catch (error) {
        console.error(
          `Validator connection failed (attempt ${i + 1}):`,
          error instanceof Error ? error.message : String(error),
        );
        if (i === maxRetries - 1) {
          throw new Error(
            `Validator did not start within the expected time after ${maxRetries} attempts.`,
          );
        }
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }

    throw new Error(
      `Validator did not start within the expected time after ${maxRetries} attempts.`,
    );
  }

  before(async function () {
    await cleanupProcesses();
  });

  afterEach(async function () {
    await cleanupProcesses();
  });

  it("should start validator without indexer and prover", async function () {
    const { stdout } = await exec(
      "./test_bin/dev test-validator --skip-indexer --skip-prover",
    );
    expect(stdout).to.contain("Setup tasks completed successfully");

    console.log("Validator setup completed. Waiting for it to be ready...");

    const connection = await waitForValidatorReady();
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");
    console.log("Validator is running and verified.");

    try {
      await fetch(`http://localhost:${defaultIndexerPort}/health`, {
        signal: AbortSignal.timeout(2000),
      });
      throw new Error("Indexer should not be running");
    } catch (error) {
      expect(error).to.exist;
      console.log("Indexer is not running as expected.");
    }

    try {
      await fetch(`http://localhost:${defaultProverPort}/health`, {
        signal: AbortSignal.timeout(2000),
      });
      throw new Error("Prover should not be running");
    } catch (error) {
      expect(error).to.exist;
      console.log("Prover is not running as expected.");
    }
  });

  it("should start validator with default settings", async function () {
    console.log("Starting test-validator...");

    const { stdout } = await exec("./test_bin/dev test-validator");
    console.log("Command output:", stdout);

    expect(stdout).to.contain("Setup tasks completed successfully");
    console.log("Stdout check passed");

    console.log("Waiting for validator to be fully ready...");
    const connection = await waitForValidatorReady();

    try {
      console.log("Getting validator version...");
      const version = await connection.getVersion();
      console.log("Validator version:", version);
      expect(version).to.have.property("solana-core");

      console.log("Getting wallet balance...");
      const payer = defaultSolanaWalletKeypair();
      const balance = await connection.getBalance(payer.publicKey);
      console.log("Wallet balance:", balance);
      expect(balance).to.be.at.least(0);

      console.log("Test completed successfully");
    } catch (error) {
      console.error("Error during validator checks:", error);
      throw error;
    }
  });

  it("should start validator with custom ports", async function () {
    const command = [
      "./test_bin/dev test-validator",
      `--rpc-port ${customRpcPort}`,
      `--indexer-port ${customIndexerPort}`,
      `--prover-port ${customProverPort}`,
    ].join(" ");

    const { stdout } = await exec(command);
    expect(stdout).to.contain("Setup tasks completed successfully");

    const connection = await waitForValidatorReady(customRpcPort);
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");

    let indexerReady = false;
    for (let i = 0; i < 10; i++) {
      try {
        const indexerResponse = await fetch(
          `http://localhost:${customIndexerPort}/health`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({}),
            signal: AbortSignal.timeout(3000),
          },
        );
        expect(indexerResponse.status).to.equal(200);
        indexerReady = true;
        break;
      } catch (error) {
        console.log(`Indexer not ready, attempt ${i + 1}/10...`);
        await new Promise((resolve) => setTimeout(resolve, 2000));
      }
    }
    expect(indexerReady).to.be.true;

    let proverReady = false;
    for (let i = 0; i < 10; i++) {
      try {
        const proverResponse = await fetch(
          `http://localhost:${customProverPort}/health`,
          { signal: AbortSignal.timeout(3000) },
        );
        expect(proverResponse.status).to.equal(200);
        proverReady = true;
        break;
      } catch (error) {
        console.log(`Prover not ready, attempt ${i + 1}/10...`);
        await new Promise((resolve) => setTimeout(resolve, 2000));
      }
    }
    expect(proverReady).to.be.true;
  });

  it("should fail with invalid geyser config path", async function () {
    try {
      await exec(
        "./test_bin/dev test-validator --geyser-config nonexistent.json",
      );
      throw new Error("Should have failed");
    } catch (error) {
      const execError = error as ExecException & {
        stdout?: string;
        stderr?: string;
      };
      const errorText = execError.message || execError.stderr || "";
      expect(errorText).to.contain("Geyser config file not found");
    }
  });

  it("should start and stop validator with custom arguments", async function () {
    const startCommand =
      './test_bin/dev test-validator --validator-args "--log-messages-bytes-limit 1000"';
    const { stdout: startOutput } = await exec(startCommand);
    expect(startOutput).to.contain("Setup tasks completed successfully");

    const connection = await waitForValidatorReady();
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");

    const stopCommand = "./test_bin/dev test-validator --stop";
    const { stdout: stopOutput } = await exec(stopCommand);
    expect(stopOutput).to.contain("Test validator stopped successfully");

    await new Promise((resolve) => setTimeout(resolve, 3000));

    try {
      await Promise.race([
        connection.getVersion(),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error("Timeout")), 2000),
        ),
      ]);
      throw new Error("Validator should be stopped");
    } catch (error) {
      expect(error).to.exist;
    }
  });

  const SYSTEM_PROGRAM_ID = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
  const SYSTEM_PROGRAM_NAME = "spl_noop.so";

  it("should fail when deploying program with system program address", async function () {
    const testProgramPath = path.join(programsDirPath(), "test_program.so");
    fs.writeFileSync(testProgramPath, "dummy program data");

    try {
      const command = [
        "./test_bin/dev test-validator",
        "--sbf-program",
        SYSTEM_PROGRAM_ID,
        testProgramPath,
      ].join(" ");

      await exec(command);
      throw new Error(
        "Should have failed due to system program address collision",
      );
    } catch (error) {
      const execError = error as ExecException & {
        stdout?: string;
        stderr?: string;
      };
      const errorText = execError.message || execError.stderr || "";
      expect(errorText).to.contain("collides with system program");
      expect(errorText).to.contain(SYSTEM_PROGRAM_ID);
    } finally {
      fs.unlinkSync(testProgramPath);
    }
  });

  it("should fail when deploying program with system program name", async function () {
    const testProgramPath = path.join(programsDirPath(), SYSTEM_PROGRAM_NAME);
    fs.writeFileSync(testProgramPath, "dummy program data");

    try {
      const testKeypair = Keypair.generate();
      const command = [
        "./test_bin/dev test-validator",
        "--sbf-program",
        testKeypair.publicKey.toString(),
        testProgramPath,
      ].join(" ");

      await exec(command);
      throw new Error(
        "Should have failed due to system program name collision",
      );
    } catch (error) {
      const execError = error as ExecException & {
        stdout?: string;
        stderr?: string;
      };
      const errorText = execError.message || execError.stderr || "";
      expect(errorText).to.contain("collides with system program");
      expect(errorText).to.contain(SYSTEM_PROGRAM_NAME);
    } finally {
      fs.unlinkSync(testProgramPath);
    }
  });

  it("should fail when deploying multiple programs with same address", async function () {
    const testProgramPath1 = path.join(programsDirPath(), "test_program1.so");
    const testProgramPath2 = path.join(programsDirPath(), "test_program2.so");
    fs.writeFileSync(testProgramPath1, "dummy program data 1");
    fs.writeFileSync(testProgramPath2, "dummy program data 2");

    try {
      const testKeypair = Keypair.generate();
      const command = [
        "./test_bin/dev test-validator",
        "--sbf-program",
        testKeypair.publicKey.toString(),
        testProgramPath1,
        "--sbf-program",
        testKeypair.publicKey.toString(),
        testProgramPath2,
      ].join(" ");

      await exec(command);
      throw new Error("Should have failed due to duplicate program address");
    } catch (error) {
      const execError = error as ExecException & {
        stdout?: string;
        stderr?: string;
      };
      const errorText = execError.message || execError.stderr || "";
      expect(errorText).to.contain("Duplicate program address detected");
    } finally {
      fs.unlinkSync(testProgramPath1);
      fs.unlinkSync(testProgramPath2);
    }
  });

  it("should succeed with valid program deployment avoiding system collisions", async function () {
    const testProgramPath = path.join(programsDirPath(), "custom_program.so");
    fs.writeFileSync(testProgramPath, "dummy program data");

    try {
      const testKeypair = Keypair.generate();
      const command = [
        "./test_bin/dev test-validator",
        "--sbf-program",
        testKeypair.publicKey.toString(),
        testProgramPath,
      ].join(" ");

      const { stdout } = await exec(command);
      expect(stdout).to.contain("Setup tasks completed successfully");

      const connection = await waitForValidatorReady();
      const version = await connection.getVersion();
      expect(version).to.have.property("solana-core");

      let programAccountInfo = await connection.getAccountInfo(
        testKeypair.publicKey,
      );
      expect(programAccountInfo).to.exist;
      expect(programAccountInfo!.executable).to.be.true;
    } finally {
      fs.unlinkSync(testProgramPath);
    }
  });
});
