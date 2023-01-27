import { log } from "../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { PublicKey } from "@solana/web3.js";
import { getAirDrop, getLocalProvider, getWalletConfig, readPayerFromIdJson } from "../../utils/utils"
import { Command } from "commander";


export const initialize = new Command("initialize").argument("-p, --pubKey <pubKey>")
  .description("initialize the Merkle Tree Authority")
  .action(async (command: string, options: any) => {
    // Start the loading spinner
    const MERKLE_TREE_KEY = new PublicKey(command)
    const spinner = ora('Merkle Tree Account\n').start();
    try {
      const payer = new anchor.Wallet(readPayerFromIdJson());
      const provider = await getLocalProvider(payer);
      console.log(anchor.web3.Keypair.generate().publicKey)
      await getAirDrop(provider, payer.publicKey)
      const merkleTreeAccountInfo = await provider.connection.getAccountInfo(MERKLE_TREE_KEY);
      if (!merkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY, readPayerFromIdJson())
        log("Initializing new Merkle Tree Account", "info");
        try {
          console.log(merkleTreeConfig.merkleTreePubkey)
          const ix = await merkleTreeConfig.initializeNewMerkleTree();
          spinner.succeed('Merkle Tree Account initialized successfully');
        } catch (error) {
          throw error
        }
      } else {
        log('Merkle Tree Account already exists', "info");
      }
      spinner.stop();
    } catch (error) {
      spinner.fail('Error initializing Merkle Tree Account');
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      log(errorMessage, "error");
    }
  })