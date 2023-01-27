import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MerkleTreeConfig, confirmConfig, verifierProgramOneProgramId, verifierProgramTwoProgramId, verifierProgramZeroProgramId } from "light-sdk";
import { log } from "../../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig } from "../../../utils/utils";
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";


export const verifier = new Command("verifier").argument("method")
    .description("Initialize or get the Merkle Tree Verifier Account")
    .option("-p, --publicKey <pubKey>", "Public key of the Verifier")
    .description("Register a new verifier for a Merkle Tree")
    .action(async (command: string, options: any) => {
        const verifierKey = new PublicKey(program.args[2])
        if (command == "init") {
            // const spinner = ora({
            //     text: "Registering Verifiers...",
            //     spinner: "dots",
            //     color: "green",
            // });
            try {
                // spinner.start();
                const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
                const provider = await getLocalProvider(payer);
                let merkleTreeConfig = await getWalletConfig(provider);

                try {
                    await merkleTreeConfig.registerVerifier(new PublicKey(verifierKey));
                    console.log(`Registering Verifier ${verifierKey} success`);
                } catch (err) {
                    console.log(`Error while registering verifier ${verifierKey}`)
                    throw err
                }
                // spinner.succeed("Verifiers registered successfully!");
            } catch (error) {
                // spinner.fail("Failed to register verifiers");
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

        }
        else if (command == "get") {
            try {
                // spinner.start();
                const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
                const provider = await getLocalProvider(payer);
                let merkleTreeConfig = await getWalletConfig(provider);

                try {
                    const VerifierPdaAccount = await merkleTreeConfig.getRegisteredVerifierPda(verifierKey)
                    console.log(`Verifier :`);
                    console.log(VerifierPdaAccount)
                } catch (err) {
                    console.log(`Error while registering verifier ${verifierKey}`)
                    throw err
                }
                // spinner.succeed("Verifiers registered successfully!");
            } catch (error) {
                // spinner.fail("Failed to register verifiers");
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

        }
    })

