import {
  createNewWallet,
  getAirdrop,
  readWalletFromFile,
  saveUserToFile,
} from "../util";
import * as solana from "@solana/web3.js";
import { getLightInstance, User } from "light-sdk";
import type { Arguments, CommandBuilder } from "yargs";

type Options = {
  reset: boolean | undefined;
};

export const command: string = "register";
export const desc: string =
  "Register a light user using an existing solana wallet or with a fresh one";

export const builder: CommandBuilder<Options> = (yargs) =>
  yargs.options({
    reset: { type: "boolean" },
  });

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { reset } = argv;
  var wallet: solana.Keypair;
  if (!reset) {
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

  await getAirdrop(wallet);
  // same as login. would here optionally also register account on-chain (not strictly necessary)
  const lightInstance = await getLightInstance();
  const user = new User({ payer: wallet, lightInstance });
  await user.load();
  saveUserToFile({ user });
  console.log("User registered!", user);
  process.exit(0);
};
