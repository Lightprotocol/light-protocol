import { createNewWallet, readWalletFromFile, saveUserToFile } from "../util";
import * as solana from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { User } from "light-sdk";
import { sign } from "tweetnacl";
import { SIGN_MESSAGE } from "../constants";
import type { Arguments, CommandBuilder } from "yargs";
const message = new TextEncoder().encode(SIGN_MESSAGE);
const circomlibjs = require("circomlibjs");

export const command: string = "login";
export const desc: string =
  "login a light user using an existing solana wallet; simulates a page refresh/mount";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = async (argv: Arguments): Promise<void> => {
  var wallet: solana.Keypair;
  try {
    wallet = readWalletFromFile();
  } catch (e) {
    throw new Error(
      "No secret.txt file found, please create a new wallet with the 'new wallet' command."
    );
  }

  const signature: Uint8Array = sign.detached(message, wallet.secretKey);

  if (!sign.detached.verify(message, signature, wallet.publicKey.toBytes()))
    throw new Error("Invalid signature!");

  const signatureArray = Array.from(signature);
  // TODO: fetch and find user utxos (decr, encr)
  const decryptedUtxos: Object[] = [];
  saveUserToFile({ signature: signatureArray, utxos: decryptedUtxos });

  const poseidon = await circomlibjs.buildPoseidonOpt();
  // TODO: add utxos to user..., add balance etc all to user account, also keys,
  const user = new User(
    poseidon,
    new anchor.BN(signatureArray).toString("hex"),
    wallet
  );
  // TODO: encrypt utxos and store in "localstorage"
  console.log("User logged in!", user);
  process.exit(0);
};
