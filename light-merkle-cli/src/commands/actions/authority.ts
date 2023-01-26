import { ADMIN_AUTH_KEYPAIR, AUTHORITY, MERKLE_TREE_KEY } from "light-sdk";
import { log } from "../../../utils/logger";
import { getLocalProvider, getWalletConfig } from "../../../utils/utils";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';

// TODO: add functionality to update the new authority for the given address
// TODO: can we create new authority account for the merkle tree?
export const authority = async () => {
    // Start the loading spinner
    const spinner = ora('Initializing Merkle Tree Authority\n').start();
    log(`Creating Authority Acccount: ${AUTHORITY}`)
    try {
        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(provider);

        // Initialize the Merkle Tree Authority
        const ix = await merkleTreeConfig.initMerkleTreeAuthority();
        console.log({ ix })
        spinner.succeed(`Merkle Tree Authority initialized successfully\n`);

    } catch (error) {
        spinner.stop()
        let errorMessage = "Aborted.";
        if (error instanceof Error) {
            errorMessage = error.message;
            // @ts-ignore
            if (error.logs) {
                // @ts-ignore
                errorMessage = error.logs;
            }
        }
        log(errorMessage, "error");
    }
};