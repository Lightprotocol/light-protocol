import { log } from "../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Program } from "@coral-xyz/anchor";
import {
    ADMIN_AUTH_KEYPAIR, MerkleTreeProgram, merkleTreeProgramId,
} from "light-sdk";
import { getLocalProvider, getMerkleTreeProgram } from "../../utils/utils"
import { Command } from "commander";


export const list = new Command("list")
    .description("List all registered authority accounts, merkle tree, verifiers, pool and tokens")
    .action(async (command: string, options: any) => {
        const spinner = ora('listing Accounts\n').start();
        try {
            const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
            const provider = await getLocalProvider(payer);

            const program = new Program(
                MerkleTreeProgram,
                merkleTreeProgramId,
                provider,
            );

            // @ts-ignore
            const merkleTreeAccounts = await program.account.merkleTree.all()

            // @ts-ignore
            const authorityAccounts = await program.account.merkleTreeAuthority.all()

            // @ts-ignore
            const verifierAccounts = await program.account.registeredVerifier.all()
            // @ts-ignore
            const assetPoolsAccounts = await program.account.registeredAssetPool.all()
            // @ts-ignore
            const poolAccounts = await program.account.registeredPoolType.all()


            if (merkleTreeAccounts.length > 0) {
                log("Merkle Tree Accounts:", "success")
                console.table(merkleTreeAccounts.map((account: any) => {
                    return { pubKey: `${account.publicKey}` }
                }), ["pubKey"])
            }
            else {
                log("No merkle tree account found", "info")
            }


            if (authorityAccounts.length > 0) {
                log("Authority Accounts:", "success")
                console.table(authorityAccounts.map((account: any) => {
                    return { pubKey: `${account.publicKey}` }
                }), ["pubKey"])
            }
            else {
                log("No authority account found", "info")
            }

            if (verifierAccounts.length > 0) {
                log("Verifier Accounts:", "success")
                console.table(verifierAccounts.map((account: any) => {
                    return { pubKey: `${account.publicKey}` }
                }), ["pubKey"])
            }
            else {
                log("No verifier account found", "info")
            }


            if (assetPoolsAccounts.length > 0) {
                log("Asset Pool Accounts:", "success")
                console.table(assetPoolsAccounts.map((account: any) => {
                    return { pubKey: `${account.publicKey}` }
                }), ["pubKey"])
            }
            else {
                log("No asset pool account found", "info")

            }

            if (poolAccounts.length > 0) {
                log("Pool Accounts:", "success")
                console.table(poolAccounts.map((account: any) => {
                    return { pubKey: `${account.publicKey}` }
                }), ["pubKey"])
                console.log("\n")
            }
            else {
                log("No pool account found", "info")

            }
            spinner.succeed("Accounts information retrieved successfully");
        } catch (error) {
            spinner.fail("Error retrieving information");
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
            }
            log(errorMessage, "error");
        }
    })