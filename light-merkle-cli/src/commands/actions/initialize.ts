import { log } from "../../../utils/logger";
import * as anchor from "@coral-xyz/anchor";
import ora from 'ora';
import { Connection, LAMPORTS_PER_SOL, Keypair as SolanaKeypair } from "@solana/web3.js";
import {
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MERKLE_TREE_KEY,
} from "light-sdk";
import { getAirDrop, getLocalProvider, getWalletConfig } from "../../../utils/utils"


// TODO: support for creating a merkle tree based on new key
// TODO: support for adding custom payer for the merkle tree
// TODO: support for adding a custom authority for the merkle tree
// TODO: support for custom authority account for the merkle tree
// TODO: support for error handling if the merkle tree pda is not initialized yet below is the error code
// error: {
//   errorCode: { code: 'AccountNotInitialized', number: 3012 },
//   errorMessage: 'The program expected this account to be already initialized',
//   comparedValues: undefined,
//   origin: 'merkle_tree_authority_pda'
// },

export const initialize = async () => {
  // Start the loading spinner
  const spinner = ora('Initializing Merkle Tree Authority\n').start();

  try {
    const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
    const provider = await getLocalProvider(payer);

    await getAirDrop(provider,payer.publicKey)

    const merkleTreeAccountInfo = await provider.connection.getAccountInfo(MERKLE_TREE_KEY);
    if (!merkleTreeAccountInfo) {
      let merkleTreeConfig = await getWalletConfig(provider)

      console.log("Initializing new Merkle Tree Authority");

      try {
        const ix = await merkleTreeConfig.initializeNewMerkleTree();
        console.log({ ix });
        spinner.succeed('Merkle Tree Authority initialized successfully');

      } catch (e) {
        spinner.fail('Error initializing Merkle Tree Authority');
        console.log(e);
      }
    } else {
      spinner.info('Merkle Tree Authority already exists');
    }
  } catch (error) {
    spinner.fail('Error initializing Merkle Tree Authority');
    let errorMessage = "Aborted.";
    if (error instanceof Error) {
      errorMessage = error.message;
    }
    console.log("error ===>")
    log(errorMessage, "error");
  }
};