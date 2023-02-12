import { readUserFromFile } from "../util";
import * as solana from "@solana/web3.js";
import type { Arguments, CommandBuilder } from "yargs";
import * as anchor from "@coral-xyz/anchor";
import { strToArr } from "light-sdk";
export const command: string = "transfer";
export const desc: string =
  "create, send and confirm an transfer transaction for given, <amount> <token>, and to <recipient>";

type Options = {
  amount: number;
  token: string; // TODO: add options
  recipient: string;
  shieldedRecipient: string;
  encryptionPublicKey: string;
};
export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    amount: { type: "number" },
    token: { type: "string" },
    shieldedRecipient: { type: "string" },
    encryptionPublicKey: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = "./cache/secret.txt";

  const { amount, token, shieldedRecipient, encryptionPublicKey } = argv;
  try {
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }
  console.log("hexstring?", shieldedRecipient);
  const recipient = new anchor.BN(shieldedRecipient, "hex");
  const recipientEncryptionPublicKey: Uint8Array =
    strToArr(encryptionPublicKey);
  console.log("user.transfer...");
  await user.transfer({
    amount: amount * 1e9,
    token,
    recipient,
    recipientEncryptionPublicKey, // TODO: do shielded address
  });
  console.log(`Shielded Transfer done: ${amount} ${token}`);

  process.exit(0);
};
