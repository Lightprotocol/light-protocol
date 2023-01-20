import { createNewWallet, readWalletFromFile, saveUserToFile } from "../util";
import * as solana from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { User } from "light-sdk";
import { sign } from "tweetnacl";
import { SIGN_MESSAGE } from "../constants";
import type { Arguments, CommandBuilder } from "yargs";
const message = new TextEncoder().encode(SIGN_MESSAGE);
const circomlibjs = require("circomlibjs");

type Options = {
  clean: boolean | undefined;
};

export const command: string = "register";
export const desc: string =
  "register a light user using an existing solana wallet or with a fresh one";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    clean: { type: "boolean" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { clean } = argv;
  var wallet: solana.Keypair;
  if (!clean) {
    try {
      wallet = readWalletFromFile();
    } catch (e) {
      console.log("No wallet found, creating new wallet...");
      wallet = createNewWallet();
    }
  } else {
    console.log("Resetting wallet...");
    wallet = createNewWallet();
  }
  const signature: Uint8Array = sign.detached(message, wallet.secretKey);

  if (!sign.detached.verify(message, signature, wallet.publicKey.toBytes()))
    throw new Error("Invalid signature!");

  const signatureArray = Array.from(signature);
  // TODO: fetch and find user utxos (decr, encr)
  const decryptedUtxos: Array<Object> = [];
  saveUserToFile({ signature: signatureArray, utxos: decryptedUtxos });

  const poseidon = await circomlibjs.buildPoseidonOpt();
  // TODO: add utxos to user..., add balance etc all to user account, also keys,
  const user = new User(
    poseidon,
    new anchor.BN(signatureArray).toString("hex"),
    wallet
  );
  console.log("User registered!", user);
  process.exit(0);
};
