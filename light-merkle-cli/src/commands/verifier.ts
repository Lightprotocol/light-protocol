import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, } from "light-sdk";
import { log } from "../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig, readPayerFromIdJson } from "../../utils/utils";
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";


export const verifier = new Command("verifier").argument("method")
    .description("Initialize or get the Merkle Tree Verifier Account")
    .option("-p, --publicKey <pubKey>", "Public key of the Verifier")
    .description("Register a new verifier for a Merkle Tree")
    .action(async (command: string, options: any) => {
        const verifierKey = new PublicKey(program.args[2])
        let spinner = ora("Registering Verifiers...");
        try {
            if (command == "set") {
                spinner.start();
                const payer = new anchor.Wallet(readPayerFromIdJson());
                const provider = await getLocalProvider(payer);
                let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY, readPayerFromIdJson());
                try {
                    await merkleTreeConfig.registerVerifier(new PublicKey(verifierKey));
                    spinner.succeed("Verifiers registered successfully!");
                } catch (err) {
                    throw err
                }
            }
            else if (command == "get") {
                spinner = ora("Getting Verifier");
                spinner.start();
                const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
                const provider = await getLocalProvider(payer);
                let merkleTreeConfig = await getWalletConfig(provider);
                try {
                    const VerifierPdaAccount = await merkleTreeConfig.getRegisteredVerifierPda(verifierKey)
                    console.log(VerifierPdaAccount)
                    spinner.succeed("Verifier Successfully Logged")
                } catch (err) {
                    console.log(`Error while registering verifier ${verifierKey}`)
                    throw err
                }

            }
        } catch (error) {
            spinner.fail("Command Failed");
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
            }
            // @ts-ignore
            if (error.logs && error.logs.length > 0) {
                // @ts-ignore
                errorMessage = error.logs;
            }
            log(errorMessage, "error");
        }

    })

