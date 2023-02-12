import { readUserFromFile } from "../util";
import * as solana from "@solana/web3.js";
import type { Arguments, CommandBuilder } from "yargs";

// TODO: add custom recipient (self only atm)
export const command: string = "shield <amount> <token>";
export const desc: string =
  "create send and confirm a shield transaction for given <amount> and <token>";

type Options = {
  amount: number;
  token: string; // TODO: add options
};
export const builder: CommandBuilder<Options, Options> = (yargs) =>
  yargs
    .positional("amount", { type: "number", demandOption: true })
    .positional("token", { type: "string", demandOption: true });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { amount, token } = argv;
  try {
    var user = await readUserFromFile();
  } catch (e) {
    throw new Error("No user.txt file found, please login first.");
  }
  // TODO: ensure 'payer's' balance is enough w 'connection'

  await user.shield({ amount, token });

  process.exit(0);
};
