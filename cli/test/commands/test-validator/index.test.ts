import { expect } from "@oclif/test";
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
      // Stop the validator using CLI command first
      try {
        await exec("./test_bin/dev test-validator --stop");
        console.log("Validator stopped via CLI command");
      } catch (e) {
        console.log("No running validator to stop via CLI");
      }

      // Then force kill any remaining processes
      await killProcess("solana-test-validator");
      await killProcess("photon");
      await killProcess("prover");

      // Wait for processes to fully terminate
      await new Promise((resolve) => setTimeout(resolve, 2000));

      // Verify processes are actually stopped
      try {
        await fetch(`http://localhost:${defaultRpcPort}/health`);
        throw new Error("Validator still running");
      } catch (e) {
        console.log("Validator stopped");
      }

      try {
        await fetch(`http://localhost:${defaultIndexerPort}/health`);
        throw new Error("Indexer still running");
      } catch (e) {
        console.log("Indexer stopped");
      }

      try {
        await fetch(`http://localhost:${defaultProverPort}/health`);
        throw new Error("Prover still running");
      } catch (e) {
        console.log("Prover stopped");
      }

      console.log("Cleanup completed successfully");
    } catch (error) {
      console.error("Error in cleanup:", error);
      throw error;
    }
  }

  before(async function () {
    await cleanupProcesses();
  });

  afterEach(async function () {
    await cleanupProcesses();
  });

  it("should start validator with default settings", async function () {
    console.log("Starting test-validator...");

    const { stdout } = await exec("./test_bin/dev test-validator");
    console.log("Command output:", stdout);

    expect(stdout).to.contain("Setup tasks completed successfully");
    console.log("Stdout check passed");

    console.log("Waiting for validator to be fully ready...");
    await new Promise((resolve) => setTimeout(resolve, 5000));

    console.log("Attempting to connect to validator...");
    const connection = new Connection(`http://localhost:${defaultRpcPort}`, {
      commitment: "confirmed",
      confirmTransactionInitialTimeout: 10000,
    });

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

    await new Promise((resolve) => setTimeout(resolve, 5000));

    // Verify validator on custom port
    const connection = new Connection(`http://localhost:${customRpcPort}`);
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");

    // Verify indexer on custom port
    const indexerResponse = await fetch(
      `http://localhost:${customIndexerPort}/health`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      },
    );
    expect(indexerResponse.status).to.equal(200);

    // Verify prover on custom port
    const proverResponse = await fetch(
      `http://localhost:${customProverPort}/health`,
    );
    expect(proverResponse.status).to.equal(200);
  });

  it("should start validator without indexer and prover", async function () {
    const { stdout } = await exec(
      "./test_bin/dev test-validator --skip-indexer --skip-prover",
    );
    expect(stdout).to.contain("Setup tasks completed successfully");

    await new Promise((resolve) => setTimeout(resolve, 5000));

    // Verify validator is running
    const connection = new Connection(`http://localhost:${defaultRpcPort}`);
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");

    // Verify indexer is not running
    try {
      await fetch(`http://localhost:${defaultIndexerPort}/health`);
      throw new Error("Indexer should not be running");
    } catch (error) {
      expect(error).to.exist;
    }

    // Verify prover is not running
    try {
      await fetch(`http://localhost:${defaultProverPort}/health`);
      throw new Error("Prover should not be running");
    } catch (error) {
      expect(error).to.exist;
    }
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
      // Check either error message or stderr
      const errorText = execError.message || execError.stderr || "";
      expect(errorText).to.contain("Geyser config file not found");
    }
  });

  it("should start and stop validator with custom arguments", async function () {
    const startCommand =
      './test_bin/dev test-validator --validator-args "--log-messages-bytes-limit 1000"';
    const { stdout: startOutput } = await exec(startCommand);
    expect(startOutput).to.contain("Setup tasks completed successfully");

    await new Promise((resolve) => setTimeout(resolve, 5000));

    // Verify validator is running
    const connection = new Connection(`http://localhost:${defaultRpcPort}`);
    const version = await connection.getVersion();
    expect(version).to.have.property("solana-core");

    // Stop validator
    const stopCommand = "./test_bin/dev test-validator --stop";
    const { stdout: stopOutput } = await exec(stopCommand);
    expect(stopOutput).to.contain("Test validator stopped successfully");

    // Verify validator is stopped
    try {
      await connection.getVersion();
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
        SYSTEM_PROGRAM_ID, // Use system program address
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
        testKeypair.publicKey.toString(), // Same address as first program
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

      await new Promise((resolve) => setTimeout(resolve, 5000));

      // Verify validator is running
      const connection = new Connection(`http://localhost:${defaultRpcPort}`);
      const version = await connection.getVersion();
      expect(version).to.have.property("solana-core");
    } finally {
      fs.unlinkSync(testProgramPath);
    }
  });
});
