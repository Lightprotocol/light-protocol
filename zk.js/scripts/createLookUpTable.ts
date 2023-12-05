import {
  confirmConfig,
  initLookUpTable,
  useWallet,
} from "../src";
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair } from "@solana/web3.js";

import { PathOrFileDescriptor, readFileSync } from "fs";

process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "https://api.testnet.solana.com";
process.env.LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS = "true";

async function main() {
  const privkey = JSON.parse(
    readFileSync(process.env.ANCHOR_WALLET as PathOrFileDescriptor, "utf8"),
  );
  const payer = Keypair.fromSecretKey(Uint8Array.from(privkey));

  // Replace this with your user's Solana wallet
  const connection = new Connection(
    process.env.ANCHOR_PROVIDER_URL!,
    confirmConfig,
  );
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(payer),
    confirmConfig,
  );
  const lookupTable = await initLookUpTable(
    useWallet(payer, process.env.ANCHOR_PROVIDER_URL, true, "confirmed"),
    provider,
  );
  console.log("lookupTable ", lookupTable);
}

main();
