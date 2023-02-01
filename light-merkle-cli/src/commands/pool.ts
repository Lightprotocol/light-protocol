import ora from "ora";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MINT, MerkleTreeProgram, POOL_TYPE, merkleTreeProgramId } from "light-sdk";
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
            if (command === "pooltype") {
                const registerPoolTypeLoader = ora("Registering pool type...").start();
                try {
                    await merkleTreeConfig.registerPoolType(POOL_TYPE);
                    registerPoolTypeLoader.succeed();
                    log("Successfully registered pool type", "success")
                } catch (error) {
                    registerPoolTypeLoader.fail("Failed to register pool type");
                    throw (error)
                }
            }
            else if (command === "spl") {
                const registerSplPoolLoader = ora("Registering spl pool...").start();
                if (!program.args[2]) {
                    registerSplPoolLoader.fail("Invalid arguments pubKey required");
                    throw ("Mint pubKey required for register Spl Pool");
                }
                const mintKey = new PublicKey(program.args[2])
                try {
                    await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);
                    registerSplPoolLoader.succeed();
                    log("Successfully registered spl pool", "success")
                } catch (error) {
                    registerSplPoolLoader.fail("Failed to register sol pool");
                    throw (error)
                }
            }
            else if (command === "sol") {
                const registerSolPoolLoader = ora("Registering sol pool...").start();
                try {
                    await merkleTreeConfig.registerSolPool(POOL_TYPE);
                    registerSolPoolLoader.succeed();
                    log("Successfully registered sol pool", "success")
                } catch (error) {
                    registerSolPoolLoader.fail("Failed to register sol pool");
                    throw (error)
                }
            }
            else if (command === "list") {
                const listingPoolLoader = ora("Listing Pools");

                listingPoolLoader.start();
                const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
                const provider = await getLocalProvider(payer);
                const merkleProgram = new anchor.Program(
                    MerkleTreeProgram,
                    merkleTreeProgramId,
                    provider,
                );
                try {
                    // @ts-ignore
                    const assetPoolsAccounts = await merkleProgram.account.registeredAssetPool.all()
                    // @ts-ignore
                    const poolAccounts = await merkleProgram.account.registeredPoolType.all()
                    if (assetPoolsAccounts.length > 0) {
                        log("\nAsset Pool Accounts:", "success")
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


                    listingPoolLoader.succeed("Pools Successfully Listed")
                } catch (err) {
                    console.log(`Error while listing verifiers`)
                    throw err
                }
            }
            else {
                log("Invalid method use try using [pooltype,spl,sol,list] with pool command", "error")
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
