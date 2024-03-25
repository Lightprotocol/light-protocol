import * as fs from "fs";
import { existsSync } from "fs";
import * as path from "path";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";
import { Keypair } from "@solana/web3.js";
import { confirmConfig } from "@lightprotocol/stateless.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { CONFIG_FILE_NAME, CONFIG_PATH, DEFAULT_CONFIG } from "../psp-utils";
import { getKeypairFromFile } from "@solana-developers/helpers";
import spinner from "cli-spinners";

require("dotenv").config();

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
};

export const getSolanaRpcUrl = (): string => {
  const config = getConfig();
  return config.solanaRpcUrl;
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
