import { log } from "../../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Connection, LAMPORTS_PER_SOL, PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";
import {
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MERKLE_TREE_KEY,
} from "light-sdk";
import { getAirDrop, getLocalProvider, getWalletConfig } from "../../../utils/utils"
import { Command } from "commander";


export const initialize = new Command("initialize").argument("-p, --pubKey <pubKey>")
  .description("initialize the Merkle Tree Authority")
  .action(async (command: string, options: any) => {
    // Start the loading spinner

    const MERKLE_TREE_KEY = new PublicKey(command)

    try {
      const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
      const provider = await getLocalProvider(payer);

      await getAirDrop(provider, payer.publicKey)
      const merkleTreeAccountInfo = await provider.connection.getAccountInfo(MERKLE_TREE_KEY);
      if (!merkleTreeAccountInfo) {
        let merkleTreeConfig = await getWalletConfig(provider, MERKLE_TREE_KEY)

        log("Initializing new Merkle Tree Account", "info");

        try {
          const ix = await merkleTreeConfig.initializeNewMerkleTree();
          console.log({ ix });
          // spinner.succeed('Merkle Tree Authority initialized successfully');
          log(`Merkle Tree Authority initialized successfully`, "success");

        } catch (e) {
          log('Error initializing Merkle Tree Account', "error");

          console.log(e);
        }
      } else {
        log('Merkle Tree Account already exists', "info");
      }
    } catch (error) {
      // spinner.fail('Error initializing Merkle Tree Authority');
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      log(errorMessage, "error");
    }
  })