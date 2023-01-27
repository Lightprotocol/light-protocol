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
import { Command, program } from "commander";


export const configure = new Command("configure").argument("method")
    .description("Update the configuration of the merkle Tree nfts , permissions less spl tokens and lock duration")
    .option("-l , --lockDuration [lockDuration]", "Update the lockDuration configuration")
    .description("initialize the Merkle Tree Authority")
    .action(async (command: string, options: any) => {

        console.log(program.args, command, options.enableNfts)
        const spinner = ora('Updating Merkle Tree Configuration...\n').start();

        try {
            spinner.start("Updating Merkle Tree Configuration...");
            const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
            const provider = await getLocalProvider(payer);
            let merkleTreeConfig = await getWalletConfig(provider)

            if (command === "nfts") {
                await merkleTreeConfig.enableNfts(true);
            }

            if (command === "enablePermissionlessSplTokens") {
                await merkleTreeConfig.enablePermissionlessSplTokens(true);
            }

            if (command === "lockDuration") {
                await merkleTreeConfig.updateLockDuration(parseInt(program.args[2]));
            }

            spinner.succeed("Merkle Tree Configuration updated successfully");
        } catch (error) {
            spinner.fail("Error updating Merkle Tree Configuration");
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
            }
            log(errorMessage, "error");
        }

    })