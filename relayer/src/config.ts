import { ADMIN_AUTH_KEYPAIR } from "light-sdk";
import * as solana from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";

export const relayerPayer = ADMIN_AUTH_KEYPAIR;
export const relayerFeeRecipient = solana.Keypair.generate();
export const relayerFee = new anchor.BN(100000);

export const rpcPort = 8899;
export const port = 3331;