import * as anchor from "@coral-xyz/anchor";
import {
  executeUpdateMerkleTreeTransactions,
  SolMerkleTree,
} from "../merkleTree/index";
import { merkleTreeProgramId, MERKLE_TREE_KEY } from "../constants";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
const circomlibjs = require("circomlibjs");
import { ADMIN_AUTH_KEYPAIR } from "./constants_system_verifier";
import { Provider } from "wallet";
import { Connection } from "@solana/web3.js";

export async function updateMerkleTreeForTest(
  connection: Connection,
  provider?: Provider,
) {
  try {
    const merkleTreeProgram = new anchor.Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      // @ts-ignore
      provider && provider,
    );

    // fetch uninserted utxos from chain
    let leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      MERKLE_TREE_KEY,
      // @ts-ignore
      provider && provider,
    );

    let poseidon = await circomlibjs.buildPoseidonOpt();

    //@ts-ignore
    await executeUpdateMerkleTreeTransactions({
      connection,
      signer: ADMIN_AUTH_KEYPAIR,
      merkleTreeProgram,
      leavesPdas,
      merkle_tree_pubkey: MERKLE_TREE_KEY,
    });
    console.log("updateMerkleTreeForTest done");
  } catch (err) {
    console.error("failed at updateMerkleTreeForTest", err);
    throw err;
  }
}
