import { log } from "../../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Connection, LAMPORTS_PER_SOL, Keypair as SolanaKeypair } from "@solana/web3.js";
import {
    ADMIN_AUTH_KEYPAIR,
    AUTHORITY,
    MERKLE_TREE_KEY,
} from "light-sdk";
import { getLocalProvider, getWalletConfig } from "../../../utils/utils"

// TODO: error handling if the merkle tree is not available
// TODO: support to log multiple merkle trees at a time
// TODO: better way to showcase the merkle trees 

export const print = async () => {

    const spinner = ora('Retrieving Merkle Tree Information...\n').start();
    try {
        spinner.start("Retrieving Merkle Tree Information...");
        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        let merkleTreeAccountInfo = await provider.connection.getAccountInfo(MERKLE_TREE_KEY);
        let merkleTreeConfig = await getWalletConfig(provider)
        await merkleTreeConfig.printMerkleTree();
        spinner.succeed("Merkle Tree Information retrieved successfully");
        console.log("Merkle Tree Information: ", merkleTreeAccountInfo);
    } catch (error) {
        spinner.fail("Error retrieving Merkle Tree Information");
        let errorMessage = "Aborted.";
        if (error instanceof Error) {
            errorMessage = error.message;
        }
        log(errorMessage, "error");
    }
}