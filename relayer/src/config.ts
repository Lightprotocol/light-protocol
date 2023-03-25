import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";
import { ADMIN_AUTH_KEYPAIR } from "light-sdk";


export const relayerPayer = ADMIN_AUTH_KEYPAIR;
export const relayerFeeRecipient = solana.Keypair.generate();
export const relayerFee = new anchor.BN(100000);
export const rpcPort = 8899;