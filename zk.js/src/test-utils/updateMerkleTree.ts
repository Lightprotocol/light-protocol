import * as anchor from "@coral-xyz/anchor";
import {
  closeMerkleTreeUpdateState,
  executeUpdateMerkleTreeTransactions,
  getMerkleTreeUpdateStatePda,
  MerkleTreeConfig,
  SolMerkleTree,
} from "../merkleTree/index";
import {
  confirmConfig,
  FETCH_QUEUED_LEAVES_RETRIES,
  merkleTreeProgramId,
  UPDATE_MERKLE_TREE_RETRIES,
} from "../constants";
import { IDL_LIGHT_MERKLE_TREE_PROGRAM } from "../idls/index";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { noAtomicMerkleTreeUpdates, sleep } from "../index";
import { LightMerkleTreeProgram } from "../../src";

async function retryOperation(
  operation: () => Promise<void>,
  handleError: () => Promise<void>,
  retries = 1,
) {
  for (let i = 0; i < retries; i++) {
    try {
      await operation();
      break;
    } catch (err) {
      if (i === retries - 1) {
        throw err;
      }
      await handleError();
    }
  }
}

async function getLeavesPdas(
  transactionMerkleTreePda: PublicKey,
  anchorProvider: anchor.AnchorProvider,
) {
  let leavesPdas: any[] = [];
  let retries = FETCH_QUEUED_LEAVES_RETRIES;

  while (leavesPdas.length === 0 && retries > 0) {
    if (retries !== FETCH_QUEUED_LEAVES_RETRIES) await sleep(1000);
    leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      transactionMerkleTreePda,
      anchorProvider,
    );
    retries--;
  }

  return leavesPdas;
}

/**
 * close the update state account if it exists, else don't handle the error
 */
async function handleUpdateMerkleTreeError(
  merkleTreeProgram: anchor.Program<LightMerkleTreeProgram>,
  payer: Keypair,
  connection: Connection,
) {
  const merkleTreeUpdateState = getMerkleTreeUpdateStatePda(
    payer.publicKey,
    merkleTreeProgram,
  );
  const isInited = await connection.getAccountInfo(merkleTreeUpdateState);

  if (!isInited) throw new Error("update state account not initialized");

  console.log("closing update state account...");
  await closeMerkleTreeUpdateState(merkleTreeProgram, payer, connection);
  console.log("successfully closed update state account");
}

export async function updateMerkleTreeForTest(payer: Keypair, url: string) {
  if (noAtomicMerkleTreeUpdates()) {
    throw Error(
      "This function shouldn't be called with atomic transactions enabled",
    );
  }
  const connection = new Connection(url, confirmConfig);

  const anchorProvider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(Keypair.generate()),
    confirmConfig,
  );

  const merkleTreeProgram = new anchor.Program(
    IDL_LIGHT_MERKLE_TREE_PROGRAM,
    merkleTreeProgramId,
    anchorProvider && anchorProvider,
  );

  const transactionMerkleTreePda =
    MerkleTreeConfig.getTransactionMerkleTreePda();

  const leavesPdas = await getLeavesPdas(
    transactionMerkleTreePda,
    anchorProvider,
  );
  if (leavesPdas.length === 0) throw new Error("didn't find any leaves");

  await retryOperation(
    async () => {
      await executeUpdateMerkleTreeTransactions({
        connection,
        signer: payer,
        merkleTreeProgram,
        leavesPdas,
        transactionMerkleTree: transactionMerkleTreePda,
      });
    },
    async () => {
      await handleUpdateMerkleTreeError(merkleTreeProgram, payer, connection);
    },
    UPDATE_MERKLE_TREE_RETRIES,
  );
}
