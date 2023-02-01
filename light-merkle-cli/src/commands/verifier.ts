import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MerkleTreeProgram, merkleTreeProgramId, } from "light-sdk";
import { log } from "../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig, readPayerFromIdJson } from "../../utils/utils";
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";


export const verifier = new Command("verifier").argument("method")
    .description("Initialize or get the Merkle Tree Verifier Account")
    .option("-p, --publicKey <pubKey>", "Public key of the Verifier")
    .description("Register a new verifier for a Merkle Tree")
    .action(async (command: string, options: any) => {
        let spinner = ora("Registering Verifiers...");
        try {
            if (command == "set") {
                const verifierKey = new PublicKey(program.args[2])
                spinner.start();
                const payer = new anchor.Wallet(readPayerFromIdJson());
                const provider = await getLocalProvider(payer);
                let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY, readPayerFromIdJson());
                try {
                    await merkleTreeConfig.registerVerifier(new PublicKey(verifierKey));
                    spinner.succeed("Verifiers registered successfully!");
                    log(`Verifier PubKey: ${new PublicKey(verifierKey)}\n`, "success")
                } catch (err) {
                    throw err
                }
            }
            else if (command == "get") {
                const verifierKey = new PublicKey(program.args[2])
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
            else if (command === "list") {
                spinner = ora("Listing Verifier");

                spinner.start();
                const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
                const provider = await getLocalProvider(payer);
                const merkleProgram = new Program(
                    MerkleTreeProgram,
                    merkleTreeProgramId,
                    provider,
                );
                try {
                    // @ts-ignore
                    const verifierAccounts = await merkleProgram.account.registeredVerifier.all()

                    if (verifierAccounts.length > 0) {
                        log("\nVerifier Accounts:", "success")
                        console.table(verifierAccounts.map((account: any) => {
                            return { pubKey: `${account.publicKey}` }
                        }), ["pubKey"])
                    }
                    else {
                        log("No verifier account found", "info")
                    }

                    spinner.succeed("Verifiers Successfully Listed")
                } catch (err) {
                    console.log(`Error while listing verifiers`)
                    throw err
                }
            }
            else {
                spinner.stop()
                log("Invalid command. Please use 'set', 'get' or 'list'", "error");
            }
            spinner.stop()
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

