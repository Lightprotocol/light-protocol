import * as anchor from "@coral-xyz/anchor";
import {
  executeUpdateMerkleTreeTransactions,
  SolMerkleTree,
} from "../merkleTree/index";
import {
  confirmConfig,
  merkleTreeProgramId,
  TRANSACTION_MERKLE_TREE_KEY,
} from "../constants";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
const circomlibjs = require("circomlibjs");
import { ADMIN_AUTH_KEYPAIR } from "./constants_system_verifier";
import { Provider, Wallet } from "../wallet";
import { Connection, Keypair } from "@solana/web3.js";

export async function updateMerkleTreeForTest(payer: Keypair, url: string) {
  const connection = new Connection(url, confirmConfig);
  const anchorProvider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(Keypair.generate()),
    confirmConfig,
  );
  try {
    const merkleTreeProgram = new anchor.Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      anchorProvider && anchorProvider,
    );

    // fetch uninserted utxos from chain
    let leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      TRANSACTION_MERKLE_TREE_KEY,
      anchorProvider && anchorProvider,
    );

    await executeUpdateMerkleTreeTransactions({
      connection,
      signer: payer,
      merkleTreeProgram,
      leavesPdas,
      transactionMerkleTree: TRANSACTION_MERKLE_TREE_KEY,
    });
  } catch (err) {
    console.error("failed at updateMerkleTreeForTest", err);
    throw err;
  }
}
