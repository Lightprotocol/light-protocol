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
import { IDL_MERKLE_TREE_PROGRAM } from "../idls/index";
import { Connection, Keypair } from "@solana/web3.js";
import { sleep } from "../index";

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

    let leavesPdas: any[] = [];
    let retries = 3;
    while (leavesPdas.length === 0 && retries > 0) {
      if (retries !== 3) await sleep(1000);
      leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
        TRANSACTION_MERKLE_TREE_KEY,
        anchorProvider && anchorProvider,
      );
      retries--;
    }

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
