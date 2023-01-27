import { getAirdrop, readWalletFromFile, saveUserToFile } from "../util";
import * as solana from "@solana/web3.js";
import { getLightInstance, User } from "light-sdk";
import type { Arguments, CommandBuilder } from "yargs";

export const command: string = "login";
export const desc: string =
  "login a light user using an existing solana wallet; simulates a page refresh/mount";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = async (): Promise<void> => {
  var wallet: solana.Keypair;
  try {
    wallet = readWalletFromFile();
  } catch (e) {
    throw new Error(
      "No secret.txt file found, please create a new wallet with the 'new wallet' command."
    );
  }
  await getAirdrop(wallet);

  const lightInstance = await getLightInstance();
  const user = new User({ payer: wallet, lightInstance });
  await user.load();
  saveUserToFile({ user });

  console.log("User logged in!", user);
  process.exit(0);
};
