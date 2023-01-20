import {
  createNewWallet,
  readUserFromFile,
  readWalletFromFile,
  saveUserToFile,
} from "../util";
import * as solana from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { User } from "light-sdk";
import { sign } from "tweetnacl";
import { SIGN_MESSAGE } from "../constants";
import type { Arguments, CommandBuilder } from "yargs";
const message = new TextEncoder().encode(SIGN_MESSAGE);
const circomlibjs = require("circomlibjs");

export const command: string = "login";
export const desc: string =
  "login a light user using an existing solana wallet; simulates a page refresh/mount";

export const builder: CommandBuilder = (yargs) => yargs;

export const handler = async (argv: Arguments): Promise<void> => {
  var user; // TODO: add user type
  try {
    user = readUserFromFile();
  } catch (e) {
    throw new Error("No signature.txt file found, please 'login' first.");
  }

  process.exit(0);
};
