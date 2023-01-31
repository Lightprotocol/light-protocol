import { MERKLE_TREE_AUTHORITY_PDA, MERKLE_TREE_KEY } from "light-sdk";
import { log } from "../../utils/logger";
import { getAirDrop, getLocalProvider, getWalletConfig, readPayerFromIdJson } from "../../utils/utils";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";

export const authority = new Command("authority").argument("method")
    .description("Initialize, set or get the Merkle Tree Authority")
    .option("-p, --publicKey <pubKey>", "Public key of the authority")
    .action(async (command: string, options: any) => {
        // @ts-ignore
        // Start the loading spinner
        const spinner = ora('Merkle Tree Authority\n').start();
        try {
            spinner.start();
            const payer = new anchor.Wallet(readPayerFromIdJson());
            const provider = await getLocalProvider(payer);
            await getAirDrop(provider, payer.publicKey)
            let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY, readPayerFromIdJson())
            if (command === "init") {
                spinner.stop();
                const initSpinner = ora('Initializing Merkle Tree Authority\n').start();

                try {
                    const ix = await merkleTreeConfig.initMerkleTreeAuthority();
                    initSpinner.succeed(`Merkle Tree Authority initialized successfully\n`);
                    log(`Merkle Tree Authority PubKey: ${MERKLE_TREE_AUTHORITY_PDA}\n`, "success")
                }
                catch (error) {
                    initSpinner.stop();
                    throw error;
                }
            } else if (command === "set") {
                const setSpinner = ora('Updating Merkle Tree Authority\n').start();
                spinner.stop()
                log(`Updating Authority Acccount`, "info")
                if (!program.args[2]) {
                    setSpinner.stop()
                    log("Please provide the public key of the new authority account", "error");
                    return;
                }
                try {
                    await merkleTreeConfig.updateMerkleTreeAuthority(new PublicKey(program.args[2]), true);
                    log(`updated authority: ${new PublicKey(program.args[2])}`, "success");
                    setSpinner.succeed(`Merkle Tree Authority updated successfully\n`);

                } catch (error) {
                    setSpinner.stop()
                    throw error
                }
            } else if (command === "get") {
                spinner.stop()
                const getSpinner = ora('Getting Merkle Tree Authority\n').start();
                try {
                    const authorityInfo = await provider.connection.getAccountInfo(MERKLE_TREE_AUTHORITY_PDA)
                    // @ts-ignore
                    const authority = await merkleTreeConfig.merkleTreeProgram.account.merkleTreeAuthority.fetch(
                        MERKLE_TREE_AUTHORITY_PDA,
                    );
                    console.log(`Authority Account:`, authority, "\n");
                    getSpinner.succeed(`Merkle Tree Authority retrieved successfully\n`);
                } catch (error) {
                    getSpinner.stop()
                    throw error;
                }
            } else {
                spinner.stop()
                log("Invalid command. Please use 'init', 'set' or 'get'", "error");
            }
            spinner.stop()
        } catch (error) {
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
                // @ts-ignore
                if (error.logs && error.logs.length > 0) {
                    // @ts-ignore
                    errorMessage = error.logs;
                }
            }
            spinner.stop();
            log(errorMessage, "error");
        }
    })