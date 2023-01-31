import { readUserFromFile } from "../util";
import type { Arguments, CommandBuilder } from "yargs";

export const command: string = "balance";
export const desc: string = "fetch your shielded balance";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = async (argv: Arguments): Promise<void> => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = "./cache/secret.txt";
  try {
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }
  const balances = await user.getBalance();
  console.log("User balance:");
  // for each balance, print the amount and symbol, taking into account the decimals
  balances.forEach((balance) => {
    console.log(`${balance.amount / 10 ** balance.decimals} ${balance.symbol}`);
  });

  process.exit(0);
};
