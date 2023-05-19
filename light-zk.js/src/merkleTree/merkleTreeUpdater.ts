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
  Provider,
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
    if (typeof window === "undefined") {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState
        // new anchor.BN(merkleTreeIndex) // merkle tree index
        ()
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
    } else {
      const transaction = new Transaction();
      transaction.add(
        await merkleTreeProgram.methods
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
          .signers([signer])
          .instruction(),
      );
      await sendAndConfirmTransaction(
        connection,
        transaction,
        [signer],
        confirmConfig,
      );
    }
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
    numberOfTransactions: 251,
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
    if (typeof window === "undefined") {
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
    } else {
      const transaction = new Transaction();

      transaction.add(
        await merkleTreeProgram.methods
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
          .instruction(),
      );
      await sendAndConfirmTransaction(
        connection,
        transaction,
        [signer],
        confirmConfig,
      );
    }
  } catch (e) {
    console.log(e);
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

export async function executeMerkleTreeUpdateTransactions({
  merkleTreeProgram,
  merkleTreeUpdateState,
  transactionMerkleTree,
  signer,
  numberOfTransactions,
  connection,
}: {
  merkleTreeProgram: Program<MerkleTreeProgram>;
  merkleTreeUpdateState: PublicKey;
  transactionMerkleTree: PublicKey;
  signer: Keypair;
  numberOfTransactions: number;
  connection: Connection;
}) {
  let transactions: Transaction[] = [];
  let i = 0;
  // console.log("Sending Merkle tree update transactions: ",numberOfTransactions)
  // the number of tx needs to increase with greater batchsize
  // 29 + 2 * leavesPdas.length is a first approximation

  for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
    const transaction = new Transaction();
    transaction.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    );
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    i += 1;
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    i += 1;

    transactions.push(transaction);
  }

  let error;
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

        throw err;
      }
    }),
  );
  return error;
}
