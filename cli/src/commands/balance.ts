import { readUserFromFile } from "../util";
import type { Arguments, CommandBuilder } from "yargs";

export const command: string = "balance";
export const desc: string = "fetch your shielded balance";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = async (argv: Arguments): Promise<void> => {
  try {
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }
  const balances = user.getBalance();
  console.log("User balance: ");
  balances.map((balance) =>
    console.log(`${balance.amount / balance.decimals} ${balance.symbol}`)
  );

  process.exit(0);
};
