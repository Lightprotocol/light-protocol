import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MINT, POOL_TYPE } from "light-sdk";
import { log } from "../../../utils/logger";

import * as anchor from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig } from "../../../utils/utils";
import { Command } from "commander";


export const pool = new Command("pool").argument("method")
    .description("Register a new pool type [default, spl, sol]")
    .action(async (command: string, options: any) => {

        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(provider)
        try {
            if (command === "default") {

                const registerPoolTypeLoader = ora("Registering pool type...").start();

                try {
                    await merkleTreeConfig.registerPoolType(POOL_TYPE);
                    registerPoolTypeLoader.succeed("Registering pool type success");
                } catch (e) {
                    registerPoolTypeLoader.fail("Failed to register pool type");
                    console.log(e);
                }
            }
            else if (command === "spl") {

                const registerSplPoolLoader = ora("Registering spl pool...").start();

                try {
                    await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
                    registerSplPoolLoader.succeed("Registering spl pool success");
                } catch (e) {
                    registerSplPoolLoader.fail("Failed to register spl pool");
                    console.log(e);
                }
            }
            else if (command === "sol") {
                const registerSolPoolLoader = ora("Registering sol pool...").start();
                try {
                    await merkleTreeConfig.registerSolPool(POOL_TYPE);
                    registerSolPoolLoader.succeed("Registering sol pool success");
                } catch (e) {
                    registerSolPoolLoader.fail("Failed to register sol pool");
                    console.log(e);
                }
            }
        } catch (error) {
            let errorMessage = "Aborted.";
            if (error instanceof Error) {
                errorMessage = error.message;
            }
            log(errorMessage, "error");
        }
    })
