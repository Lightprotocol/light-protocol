import { confirmConfig, initLookUpTable, useWallet } from '@lightprotocol/zk.js';
import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair } from '@solana/web3.js';

import {workspace} from "@coral-xyz/anchor"
import { PathOrFileDescriptor, readFileSync } from 'fs';
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "https://api.testnet.solana.com";

async function main() {
   
    const privkey = JSON.parse(readFileSync(process.env.ANCHOR_WALLET as PathOrFileDescriptor, "utf8"));
    const payer = Keypair.fromSecretKey(Uint8Array.from(privkey))
    
    // Replace this with your user's Solana wallet
    const connection = new Connection(process.env.ANCHOR_PROVIDER_URL, confirmConfig);
    const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(payer), confirmConfig);
    let lookupTable = await initLookUpTable(useWallet(payer, process.env.ANCHOR_PROVIDER_URL, true, "confirmed"), provider)
    console.log("lookupTable ", lookupTable);   
}

main()