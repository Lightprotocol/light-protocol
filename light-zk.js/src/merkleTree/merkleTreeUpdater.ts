import * as anchor from "@coral-xyz/anchor";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import {
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
} from "../test-utils/testChecks";

import {
  confirmConfig,
  DEFAULT_PROGRAMS,
  MerkleTreeProgram,
  sleep,
} from "../index";
import {
  ComputeBudgetProgram,
  PublicKey,
  Transaction,
  SystemProgram,
  Keypair,
  Connection,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";

export async function executeUpdateMerkleTreeTransactions({
  signer,
  merkleTreeProgram,
  leavesPdas,
  transactionMerkleTree,
  connection,
}: {
  signer: Keypair;
  merkleTreeProgram: Program<MerkleTreeProgram>;
  leavesPdas: any;
  transactionMerkleTree: PublicKey;
  connection: Connection;
}) {
  var merkleTreeAccountPrior =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTree,
    );
  let merkleTreeUpdateState = (
    await PublicKey.findProgramAddressSync(
      [
        Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
        anchor.utils.bytes.utf8.encode("storage"),
      ],
      merkleTreeProgram.programId,
    )
  )[0];
  try {
    const tx1 = await merkleTreeProgram.methods
      .initializeMerkleTreeUpdateState()
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        systemProgram: SystemProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        transactionMerkleTree: transactionMerkleTree,
      })
      .remainingAccounts(leavesPdas)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ])
      .transaction();
    await sendAndConfirmTransaction(connection, tx1, [signer], confirmConfig);
  } catch (err) {
    console.error("failed while initing the merkle tree update state", err);
    throw err;
  }

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    transactionMerkleTree: transactionMerkleTree,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 1,
    merkleTreeProgram,
  });

  await executeMerkleTreeUpdateTransactions({
    signer,
    merkleTreeProgram,
    transactionMerkleTree: transactionMerkleTree,
    merkleTreeUpdateState,
    connection,
  });

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    transactionMerkleTree: transactionMerkleTree,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 56,
    merkleTreeProgram,
  });

  try {
    const tx1 = await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        transactionMerkleTree: transactionMerkleTree,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .remainingAccounts(leavesPdas)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ])
      .transaction();
    await sendAndConfirmTransaction(connection, tx1, [signer], confirmConfig);
  } catch (e) {
    console.log(e);
    throw e;
  }

  await checkMerkleTreeBatchUpdateSuccess({
    connection: connection,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTreeAccountPrior,
    numberOfLeaves: leavesPdas.length * 2,
    leavesPdas,
    transactionMerkleTree: transactionMerkleTree,
    merkleTreeProgram,
  });
}
const createTransactions = async ({
  counter,
  merkleTreeProgram,
  numberOfTransactions,
  signer,
  merkleTreeUpdateState,
  transactionMerkleTree,
}: {
  counter: number;
  merkleTreeProgram: Program<MerkleTreeProgram>;
  numberOfTransactions: number;
  signer: Keypair;
  merkleTreeUpdateState: PublicKey;
  transactionMerkleTree: PublicKey;
}) => {
  let transactions: Transaction[] = [];
  for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
    const transaction = new Transaction();
    transaction.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    );
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(counter))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    counter += 1;
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(counter))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    counter += 1;
    transactions.push(transaction);
  }
  return { transactions, incrementedCounter: counter };
};

const checkComputeInstructionsCompleted = async (
  merkleTreeProgram: Program<MerkleTreeProgram>,
  merkleTreeUpdateState: PublicKey,
) => {
  const accountInfo =
    await merkleTreeProgram.account.merkleTreeUpdateState.fetch(
      merkleTreeUpdateState,
    );
  return accountInfo.currentInstructionIndex.toNumber() === 56;
};

const sendAndConfirmTransactions = async (
  transactions: Transaction[],
  signer: Keypair,
  connection: Connection,
) => {
  const errors: Error[] = [];
  await Promise.all(
    transactions.map(async (tx, index) => {
      try {
        await sendAndConfirmTransaction(
          connection,
          tx,
          [signer],
          confirmConfig,
        );
      } catch (err) {
        console.error(
          "failed at executing the merkle tree transaction:",
          index,
          err,
        );
        errors.push(err);
      }
    }),
  );
  if (errors.length > 0) throw errors[0];
};

export async function executeMerkleTreeUpdateTransactions({
  merkleTreeProgram,
  merkleTreeUpdateState,
  transactionMerkleTree,
  signer,
  connection,
  numberOfTransactions = 56 / 2,
  interrupt = false,
}: {
  numberOfTransactions?: number;
  merkleTreeProgram: Program<MerkleTreeProgram>;
  merkleTreeUpdateState: PublicKey;
  transactionMerkleTree: PublicKey;
  signer: Keypair;
  connection: Connection;
  interrupt?: boolean;
}) {
  /**
   * Strategy:
   * - send 28 transactions check whether update is complete
   * - if not send batches of 10 additional transactions at a time
   */
  var counter = 0;
  var error = undefined;
  while (
    !(await checkComputeInstructionsCompleted(
      merkleTreeProgram,
      merkleTreeUpdateState,
    ))
  ) {
    numberOfTransactions = counter == 0 ? numberOfTransactions : 10;
    if (counter != 0) await sleep(1000);
    const { transactions, incrementedCounter } = await createTransactions({
      numberOfTransactions,
      signer,
      counter,
      merkleTreeProgram,
      merkleTreeUpdateState,
      transactionMerkleTree,
    });
    counter = incrementedCounter;
    try {
      await sendAndConfirmTransactions(transactions, signer, connection);
    } catch (err) {
      error = err;
    }
    if (interrupt || counter >= 240) {
      console.log("Reached retry limit of 240 compute instructions");
      if (error) throw error;
      else return;
    }
  }
}
