import { log } from "../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { PublicKey } from "@solana/web3.js";
import {
    ADMIN_AUTH_KEYPAIR,
} from "light-sdk";
import { getLocalProvider, getWalletConfig } from "../../utils/utils"
import { Command } from "commander";


export const print = new Command("print").argument("-p, --pubKey <pubKey>")
    .description("Get the account information and print the merkle tree information")
    .action(async (command: string, options: any) => {
        const spinner = ora('Getting Merkle Tree Account\n').start();
        const MERKLE_TREE_KEY = new PublicKey(command)
        try {
            const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
            const provider = await getLocalProvider(payer);
            let merkleTreeAccountInfo = await provider.connection.getAccountInfo(MERKLE_TREE_KEY);
            let merkleTreeConfig = await getWalletConfig(provider)
            await merkleTreeConfig.printMerkleTree();
            console.log(merkleTreeAccountInfo)
            spinner.succeed("Merkle Tree Information retrieved successfully");
        } catch (error) {
            spinner.fail("Error retrieving Merkle Tree Information");
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
            }
            log(errorMessage, "error");
        }
    })