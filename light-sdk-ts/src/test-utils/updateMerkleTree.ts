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

export async function updateMerkleTreeForTest(
  provider: anchor.AnchorProvider,
  lightProvider: Provider,
) {
  const merkleTreeProgram = new anchor.Program(
    IDL_MERKLE_TREE_PROGRAM,
    merkleTreeProgramId,
    // @ts-ignore
    lightProvider,
  );

  // fetch uninserted utxos from chain
  let leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
    MERKLE_TREE_KEY,
    // @ts-ignore
    lightProvider,
  );

  let poseidon = await circomlibjs.buildPoseidonOpt();

  //@ts-ignore
  await executeUpdateMerkleTreeTransactions({
    connection: provider.connection,
    signer: ADMIN_AUTH_KEYPAIR,
    merkleTreeProgram,
    leavesPdas,
    merkle_tree_pubkey: MERKLE_TREE_KEY,
    provider,
  });
  console.log("updateMerkleTreeForTest done");
}
