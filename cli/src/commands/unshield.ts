import { readUserFromFile } from "../util";
import * as solana from "@solana/web3.js";
import type { Arguments, CommandBuilder } from "yargs";

export const command: string = "unshield";
export const desc: string =
  "create, send and confirm an unshield transaction for given, <amount> <token>, and to <recipient>";

type Options = {
  amount: number;
  token: string; // TODO: add options
  recipient: string;
};
export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    amount: { type: "number" },
    token: { type: "string" },
    recipient: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = "./cache/secret.txt";
  const { amount, token, recipient } = argv;
  var user;
  try {
    user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }

  await user.unshield({
    amount: Number(amount) * 1e9,
    token,
    recipient: new solana.PublicKey(recipient),
  });
  console.log(`Unhield done: ${amount} ${token} to ${recipient}`);

  process.exit(0);
};
