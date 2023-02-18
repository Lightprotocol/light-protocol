import { Program, Provider } from "@coral-xyz/anchor";
import {
  executeUpdateMerkleTreeTransactions,
  SolMerkleTree,
} from "../merkleTree/index";
import { merkleTreeProgramId, MERKLE_TREE_KEY } from "../constants";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
import { buildPoseidonOpt } from "circomlibjs";
import { ADMIN_AUTH_KEYPAIR } from "./constants_system_verifier";

export async function updateMerkleTreeForTest(provider: Provider) {
  const merkleTreeProgram = new Program(
    IDL_MERKLE_TREE_PROGRAM,
    merkleTreeProgramId,
  );

  // fetch uninserted utxos from chain
  let leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
    MERKLE_TREE_KEY,
  );

  let poseidon = await buildPoseidonOpt();
  // build tree from chain
  let mtPrior = await SolMerkleTree.build({
    pubkey: MERKLE_TREE_KEY,
    poseidon,
  });

  //@ts-ignore
  await executeUpdateMerkleTreeTransactions({
    connection: provider.connection,
    signer: ADMIN_AUTH_KEYPAIR,
    merkleTreeProgram,
    leavesPdas,
    merkleTree: mtPrior,
    merkle_tree_pubkey: MERKLE_TREE_KEY,
    provider,
  });
  console.log("updateMerkleTreeForTest done");
}
