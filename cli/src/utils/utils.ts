import * as fs from "fs";
import { existsSync } from "fs";
import * as path from "path";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";
import { Keypair } from "@solana/web3.js";
import { confirmConfig, createRpc, Rpc } from "@lightprotocol/stateless.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { CONFIG_FILE_NAME, CONFIG_PATH, DEFAULT_CONFIG } from "./constants";
import spinner from "cli-spinners";
import dotenv from "dotenv";

dotenv.config();

/**
 * Get a Keypair from a secret key file (compatible with Solana CLI)
 */
export async function getKeypairFromFile(filepath?: string): Promise<Keypair> {
  // Default value from Solana CLI
  const DEFAULT_FILEPATH = "~/.config/solana/id.json";

  if (!filepath) {
    filepath = DEFAULT_FILEPATH;
  }

  if (filepath[0] === "~") {
    const home = process.env.HOME || null;
    if (home) {
      filepath = path.join(home, filepath.slice(1));
    }
  }

  let fileContents: string;
  try {
    fileContents = fs.readFileSync(filepath, "utf8");
  } catch {
    throw new Error(`Could not read keypair from file at '${filepath}'`);
  }

  // Parse contents of file
  let parsedFileContents: Uint8Array;
  try {
    parsedFileContents = Uint8Array.from(JSON.parse(fileContents));
  } catch (error_) {
    const error = error_ as Error;
    if (!error.message.includes("Unexpected token")) {
      throw error;
    }

    throw new Error(`Invalid secret key file at '${filepath}'!`);
  }

  return Keypair.fromSecretKey(parsedFileContents);
}

export const defaultSolanaWalletKeypair = (): Keypair => {
  const walletPath = process.env.HOME + "/.config/solana/id.json";
  if (fs.existsSync(walletPath)) {
    return Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(walletPath, "utf-8"))),
    );
  } else {
    throw new Error("Wallet file not found");
  }
};

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = getWalletPath();
  process.env.ANCHOR_PROVIDER_URL = getSolanaRpcUrl();
  const connection = new solana.Connection(getSolanaRpcUrl(), "confirmed");
  const payer = await getPayer();
  const anchorProvider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(payer),
    confirmConfig,
  );

  anchor.setProvider(anchorProvider);
  return anchorProvider;
};

export function rpc(): Rpc {
  if (
    getSolanaRpcUrl() === "" ||
    getIndexerUrl() === "" ||
    getProverUrl() === ""
  ) {
    throw new Error(
      "Please set the Solana RPC URL, Indexer URL, and Prover URL in the config file",
    );
  }
  return createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());
}

function getWalletPath(): string {
  return process.env.HOME + "/.config/solana/id.json";
}

export async function getPayer() {
  return await getKeypairFromFile(getWalletPath());
}

export function generateSolanaTransactionURL(
  transactionType: "tx" | "address",
  transactionHash: string,
  cluster: string,
): string {
  return `https://explorer.solana.com/${transactionType}/${transactionHash}?cluster=${cluster}`;
}

type Config = {
  solanaRpcUrl: string;
  indexerUrl: string;
  proverUrl: string;
};

export const getSolanaRpcUrl = (): string => {
  const config = getConfig();
  return config.solanaRpcUrl;
};

export const getIndexerUrl = (): string => {
  const config = getConfig();
  return config.indexerUrl;
};

export const getProverUrl = (): string => {
  const config = getConfig();
  return config.proverUrl;
};

function getConfigPath(): string {
  // Check for the environment variable
  const envConfigPath = process.env.LIGHT_PROTOCOL_CONFIG;
  if (envConfigPath) {
    console.log(`reading config from custom path ${envConfigPath}`);
    if (!existsSync(envConfigPath)) {
      throw new Error(
        `Config file not found at ${envConfigPath}, this path is configured with the environment variable LIGHT_PROTOCOL_CONFIG, the default path is ${
          process.env.HOME + CONFIG_PATH + CONFIG_FILE_NAME
        }, to use the default path, remove the environment variable LIGHT_PROTOCOL_CONFIG`,
      );
    }
    return envConfigPath;
  }

  // Default path
  return process.env.HOME + CONFIG_PATH + CONFIG_FILE_NAME;
}

export const getConfig = (filePath?: string): Config => {
  if (!filePath) filePath = getConfigPath();
  let configData: any;
  try {
    configData = fs.readFileSync(filePath, "utf-8");
  } catch (error) {
    // Ensure the directory structure exists
    const dir = path.dirname(filePath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    ensureDirectoryExists(process.env.HOME + CONFIG_PATH);
    if (!fs.existsSync(filePath)) {
      const data = {
        ...DEFAULT_CONFIG,
        secretKey: bs58.encode(solana.Keypair.generate().secretKey),
      };

      fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
      console.log("created config file in", filePath);
      configData = fs.readFileSync(filePath, "utf-8");
    }
  }
  return JSON.parse(configData);
};

export function ensureDirectoryExists(dirPath: string): void {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

export const setConfig = (config: Partial<Config>, filePath?: string): void => {
  if (!filePath) filePath = getConfigPath();

  try {
    const existingConfig = getConfig();
    const updatedConfig = { ...existingConfig, ...config };
    fs.writeFileSync(filePath, JSON.stringify(updatedConfig, null, 2));
  } catch (error) {
    throw new Error("Failed to update configuration file");
  }
};

export class CustomLoader {
  message: string;
  logInterval: any;
  logTimer: number | null;
  startTime: number;

  constructor(message: string, logInterval = 1000) {
    this.message = message;
    this.logInterval = logInterval;
    this.logTimer = null;
    this.startTime = Date.now();
  }

  start() {
    this.startTime = Date.now();
    process.stdout.write(
      `\n${spinner.dots.frames[Math.floor(Math.random() * 10)]} ${
        this.message
      }\n`,
    );
    this.logInterval = setInterval(() => {}, this.logInterval);
  }

  stop(terminateCurve = true) {
    clearInterval(this.logInterval);
    if (terminateCurve) (globalThis as any).curve_bn128.terminate();
    this.logElapsedTime();
  }

  logElapsedTime() {
    const elapsedTime = ((Date.now() - this.startTime) / 1000).toFixed(2);
    process.stdout.write(`\nElapsed time: ${elapsedTime}s\n`);
  }
}

export function isValidURL(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch (error) {
    return false;
  }
}

export function isValidBase58SecretKey(secretKey: string): boolean {
  const base58Regex =
    /^[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]+$/;
  return base58Regex.test(secretKey);
}
