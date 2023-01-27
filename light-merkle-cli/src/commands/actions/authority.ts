import { ADMIN_AUTH_KEYPAIR, AUTHORITY, MERKLE_TREE_AUTHORITY_PDA, MERKLE_TREE_KEY } from "light-sdk";
import { log } from "../../../utils/logger";
import { getAirDrop, getLocalProvider, getWalletConfig } from "../../../utils/utils";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";

export const authority = new Command("authority").argument("method")
    .description("Initialize, set or get the Merkle Tree Authority")
    .option("-p, --publicKey <pubKey>", "Public key of the authority account")
    .action(async (command: string, options: any) => {
        // @ts-ignore
        // Start the loading spinner
        // const spinner = ora('Merkle Tree Authority\n').start();
        try {
            const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
            const provider = await getLocalProvider(payer);
            getAirDrop(provider, payer.publicKey)
            let merkleTreeConfig = await getWalletConfig(provider);
            if (command === "init") {
                // const initSpinner = ora('Initializing Merkle Tree Authority\n').start();
                try {
                    const ix = await merkleTreeConfig.initMerkleTreeAuthority();
                    log(`Merkle Tree Authority initialized successfully\n`, "success")
                    // initSpinner.succeed(`Merkle Tree Authority initialized successfully\n`);
                }
                catch (error) {
                    // initSpinner.stop();
                    throw error;
                }
            } else if (command === "set") {
                // const setSpinner = ora('Updating Merkle Tree Authority\n').start();
                log(`Updating Authority Acccount: ${AUTHORITY}`)
                if (!program.args[2]) {
                    // setSpinner.stop()
                    // spinner.stop()
                    log("Please provide the public key of the new authority account", "error");
                    return;
                }
                try {
                    await merkleTreeConfig.updateMerkleTreeAuthority(new PublicKey(program.args[2]), true);
                    console.log("Updating AUTHORITY success");
                    // setSpinner.succeed(`Merkle Tree Authority updated successfully\n`);

                } catch (error) {
                    // setSpinner.stop()
                    throw error
                }
            } else if (command === "get") {
                // const getSpinner = ora('Getting Merkle Tree Authority\n').start();
                if (!program.args[2]) {
                    // getSpinner.stop()
                    // spinner.stop()
                    log("Please provide the public key of the authority account", "error");
                    return;
                }
                try {
                    const authorityInfo = await provider.connection.getAccountInfo(new PublicKey(program.args[2]));
                    // @ts-ignore
                    const authority = await merkleTreeConfig.merkleTreeProgram.account.merkleTreeAuthority.fetch(
                        new PublicKey(program.args[2]),
                    );
                    console.log(authority);
                    console.log(`Merkle Tree Authority retrieved successfully\n`)
                    // getSpinner.succeed(`Merkle Tree Authority retrieved successfully\n`);
                } catch (error) {
                    // getSpinner.stop()
                    throw error;
                }
            } else {
                // spinner.stop()
                log("Invalid command. Please use 'init', 'set' or 'get'", "error");
            }
            // spinner.stop()
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
            console.log("error here", errorMessage)
            log(errorMessage, "error");
            // spinner.info("hello world");
            // spinner.fail(errorMessage);
        }
    })