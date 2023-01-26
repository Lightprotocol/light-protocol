import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MerkleTreeConfig, verifierProgramOneProgramId, verifierProgramTwoProgramId, verifierProgramZeroProgramId } from "light-sdk";
import { log } from "../../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig } from "../../../utils/utils";

// TODO: support error handling for seperate verifier
// TODO: support command of seleting between verifier
// TODO: Add functionality to get and set the verifier
export const verifier = async () => {
    const spinner = ora({
        text: "Registering Verifiers...",
        spinner: "dots",
        color: "green",
    });
    try {
        spinner.start();
        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(provider);

        try {
            await merkleTreeConfig.registerVerifier(verifierProgramZeroProgramId);
            console.log("Registering Verifier Zero success");
        } catch (err) { 

        }
        try {
            await merkleTreeConfig.registerVerifier(verifierProgramOneProgramId);
            console.log("Registering Verifier One success");
        } catch (err) {

        }
        try {
            await merkleTreeConfig.registerVerifier(verifierProgramTwoProgramId);
            console.log("Registering Verifier Two success");
        } catch (err) {

        }
        spinner.succeed("Verifiers registered successfully!");
    } catch (error) {
        spinner.fail("Failed to register verifiers");
        let errorMessage = "Aborted.";
        if (error instanceof Error) {
            errorMessage = error.message;
        }
        log(errorMessage, "error");
    }
};
