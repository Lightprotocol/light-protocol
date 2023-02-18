import { createNewWallet, readWalletFromFile } from "../utils";
import type { CommandBuilder } from "yargs";

export const command: string = "new wallet";
export const desc: string = "Generate a new Solana wallet (secret key)";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = (): void => {
  createNewWallet();
  readWalletFromFile();
  process.exit(0);
};
