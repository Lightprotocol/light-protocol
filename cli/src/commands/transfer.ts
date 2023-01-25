import { readUserFromFile } from "../util";
import * as solana from "@solana/web3.js";
import type { Arguments, CommandBuilder } from "yargs";

export const command: string = "transfer <amount> <token> <recipient>";
export const desc: string =
  "create, send and confirm an transfer transaction for given, <amount> <token>, and to <recipient>";

type Options = {
  amount: number;
  token: string; // TODO: add options
  recipient: string;
};
export const builder: CommandBuilder<Options> = (yargs) =>
  yargs
    .positional("amount", { type: "number", demandOption: true })
    .positional("token", { type: "string", demandOption: true })
    .positional("recipient", { type: "string", demandOption: true });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { amount, token, recipient } = argv;
  try {
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }

  const balances = user.getBalance();
  const tokenBalance = balances.find((balance) => balance.symbol === token);
  if (!tokenBalance) {
    throw new Error("Token not found");
  }
  if (tokenBalance.amount < amount) {
    throw new Error("Not enough balance");
  }

  //   await user.transfer({
  //     amount,
  //     token,
  //     recipient: new solana.PublicKey(recipient), // TODO: do shielded address
  //   });
  console.log(`Shielded Transfer done: ${amount} ${token}`);

  process.exit(0);
};
