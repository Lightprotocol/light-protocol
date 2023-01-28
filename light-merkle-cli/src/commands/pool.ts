import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MINT, POOL_TYPE } from "light-sdk";
import { log } from "../../utils/logger";

import * as anchor from "@coral-xyz/anchor";
import { getLocalProvider, getWalletConfig, readPayerFromIdJson } from "../../utils/utils";
import { Command, program } from "commander";
import { PublicKey } from "@solana/web3.js";


export const pool = new Command("pool").argument("method")
    .option("-p, --publicKey <pubKey>", "Public key for the MINT")
    .description("Register a new pool type [default, spl, sol]")
    .action(async (command: string, options: any) => {

        const payer = new anchor.Wallet(readPayerFromIdJson());
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY, readPayerFromIdJson())
        try {
            if (command === "default") {
                const registerPoolTypeLoader = ora("Registering pool type...").start();
                try {
                    await merkleTreeConfig.registerPoolType(POOL_TYPE);
                    registerPoolTypeLoader.succeed("Registering pool type success");
                } catch (error) {
                    registerPoolTypeLoader.fail("Failed to register pool type");
                    throw (error)
                }
            }
            else if (command === "spl") {
                const registerSplPoolLoader = ora("Registering spl pool...").start();
                if (!program.args[2]) {
                    registerSplPoolLoader.fail("Invalid arguments")
                    throw ("Mint pubKey required for register Spl Pool");
                }
                const mintKey = new PublicKey(program.args[2])
                try {
                    await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);
                    registerSplPoolLoader.succeed("Registering spl pool success");
                } catch (error) {
                    registerSplPoolLoader.fail("Failed to register spl pool");
                    throw (error)
                }
            }
            else if (command === "sol") {
                const registerSolPoolLoader = ora("Registering sol pool...").start();
                try {
                    await merkleTreeConfig.registerSolPool(POOL_TYPE);
                    registerSolPoolLoader.succeed("Registering sol pool success");
                } catch (error) {
                    registerSolPoolLoader.fail("Failed to register sol pool");
                    throw (error)
                }
            }
            else {
                log("Invalid method use try using [default,spl,sol] with pool command", "error")
            }
        } catch (error) {
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
