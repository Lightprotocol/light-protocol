import fsExtra from "fs-extra";
import fs, { readFileSync } from "fs";
export const indexDist = "node ../../dist/src/index.js";

import * as anchor from "@coral-xyz/anchor";
import { ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KEY, MerkleTreeConfig, confirmConfig } from "light-sdk";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { Wallet } from "@coral-xyz/anchor/dist/cjs/provider";
import { resolve } from "path";

export const fileExists = async (file: fsExtra.PathLike) => {
  return fs.promises
    .access(file, fs.constants.F_OK)
    .then(() => true)
    .catch(() => false);
};

export const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const getLocalProvider = async (payer: Wallet): Promise<anchor.AnchorProvider> => {

  return new anchor.AnchorProvider(
    await new Connection("http://127.0.0.1:8899"),
    payer,
    confirmConfig,
  );
}

//TODO: make the getWalletConfig dynamic so it can take the wallet parameters
export const getWalletConfig = async (provider: anchor.AnchorProvider, merkleTree: PublicKey = MERKLE_TREE_KEY, payer: Keypair = ADMIN_AUTH_KEYPAIR): Promise<MerkleTreeConfig> => {
  const merkleTreeConfig = new MerkleTreeConfig({
    merkleTreePubkey: merkleTree,
    payer: payer,
    connection: provider.connection,
    provider
  });

  await merkleTreeConfig.getMerkleTreeAuthorityPda()

  return merkleTreeConfig

}

export const getAirDrop = async (provider: anchor.AnchorProvider, pubkey: anchor.web3.PublicKey) => {
  // Request and confirm the airdrop
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(pubkey, LAMPORTS_PER_SOL),
    "confirmed",
  );
}

export const readPayerFromIdJson = (): any => {
  try {
    // Read the id.json file located in the root directory
    const idJson = JSON.parse(readFileSync(resolve(process.cwd(), 'id.json'), 'utf8'));
    // Return the payer object
    return Keypair.fromSecretKey(
      new Uint8Array(idJson.payer),
    );
  } catch (error) {
    console.error(`Error reading payer from id.json: ${error}`);
  }
};