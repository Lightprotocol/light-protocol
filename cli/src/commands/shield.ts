import { readUserFromFile } from "../utils";
import * as solana from "@solana/web3.js";
import type { Arguments, CommandBuilder } from "yargs";

// TODO: add custom recipient (self only atm)
export const command: string = "shield";
export const desc: string =
  "create send and confirm a shield transaction for given <amount> and <token>";

type Options = {
  amount: number;
  token: string;
};
export const builder: CommandBuilder = (yargs) =>
  yargs.options({
    amount: { type: "number" },
    token: { type: "string" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = "./light-test-cache/secret.txt";
  const { amount, token } = argv;
  try {
    // get user from state
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }
  // return;
  // TODO: ensure 'payer's' balance is enough w 'connection'
  // TODO: add check for amount / bn; optionally pass as bn
  // TODO: use tokenRegistry to find decimals and convert. <- can put behind.
  // at multiplication it should be a bn number
  // TODO: recipient
  await user.shield({ amount, token });

  console.log(`Shielding done: ${amount} ${token}`);
  process.exit(0);
};
