import * as anchor from "@coral-xyz/anchor";
import {
  verifierProgramZeroProgramId,
  verifierProgramOneProgramId,
  verifierProgramTwoProgramId,
  verifierProgramStorageProgramId,
} from "@lightprotocol/zk.js";
import "dotenv/config.js";

export const relayerFee = new anchor.BN(100000);
export const port = Number(process.env.PORT) || 3331;
export const SECONDS = 1000;
export const MINUTE = 60 * SECONDS;
export const HOUR = 60 * MINUTE;
export const TX_BATCH_SIZE = 100;
export const FORWARD_SEARCH_BATCH_SIZE = 1000;
export const DB_VERSION = 9;
export const CONCURRENT_RELAY_WORKERS = 2;
export const MAX_STEPS_TO_WAIT_FOR_JOB_COMPLETION = 60;
export enum Network {
  MAINNET = "MAINNET",
  DEVNET = "DEVNET",
  LOCALNET = "LOCALNET",
  TESTNET = "TESTNET",
}
export enum Environment {
  PROD = "PROD",
  STAGING = "STAGING",
  LOCAL = "LOCAL",
}

export enum TransactionType {
  SHIELD = "SHIELD",
  UNSHIELD = "UNSHIELD",
  TRANSFER = "TRANSFER",
}

export const NETWORK = process.env.NETWORK;
export const ENVIRONMENT = process.env.ENVIRONMENT;
export const AIRDROP_DECIMALS =
  NETWORK === Network.TESTNET ? 1e6 : NETWORK === Network.LOCALNET ? 1e9 : 0;
console.log("ENVIRONMENT", ENVIRONMENT);
export const RPC_URL = process.env.RPC_URL!;
console.log("RPC_URL", RPC_URL);

export const PORT = process.env.DB_PORT!;
export const PASSWORD = process.env.PASSWORD!;
export const HOST = process.env.HOSTNAME!;

console.log("DB PORT", PORT);
console.log("PASS: ", PASSWORD);
console.log("HOST:", HOST);
console.log("port", port);

// TODO: EXPORT FROM ZK.JS RELEASE
export const VERIFIER_PUBLIC_KEYS = [
  verifierProgramZeroProgramId,
  verifierProgramOneProgramId,
  verifierProgramTwoProgramId,
  verifierProgramStorageProgramId,
];
export const MAX_U64 = new anchor.BN("18446744073709551615");
