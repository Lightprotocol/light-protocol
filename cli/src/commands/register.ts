import {
  createNewWallet,
  getAirdrop,
  readWalletFromFile,
  saveUserToFile,
} from "../utils";
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
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = "./light-test-cache/secret.txt";
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
  await user.init();
  saveUserToFile({ user });
  console.log("User registered!");
  process.exit(0);
};
