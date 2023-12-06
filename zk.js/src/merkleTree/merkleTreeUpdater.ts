import * as anchor from "@coral-xyz/anchor";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import {
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
} from "../test-utils/testChecks";

import {
  confirmConfig,
  DEFAULT_PROGRAMS,
  LightMerkleTreeProgram,
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

export async function closeMerkleTreeUpdateState(
  merkleTreeProgram: Program<LightMerkleTreeProgram>,
  signer: Keypair,
  connection: Connection,
) {
  try {
    const tx1 = await merkleTreeProgram.methods
      .closeMerkleTreeUpdateState()
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: getMerkleTreeUpdateStatePda(
          signer.publicKey,
          merkleTreeProgram,
        ),
      })

      .transaction();
    await sendAndConfirmTransaction(connection, tx1, [signer], confirmConfig);
  } catch (e) {
    console.log(e);
    throw e;
  }
}

export function getMerkleTreeUpdateStatePda(
  authority: PublicKey,
  merkleTreeProgram: Program<LightMerkleTreeProgram>,
) {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(new Uint8Array(authority.toBytes())),
      anchor.utils.bytes.utf8.encode("storage"),
    ],
    merkleTreeProgram.programId,
  )[0];
}

export async function executeUpdateMerkleTreeTransactions({
  signer,
  merkleTreeProgram,
  leavesPdas,
  transactionMerkleTree,
  connection,
}: {
  signer: Keypair;
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
  leavesPdas: any;
  transactionMerkleTree: PublicKey;
  connection: Connection;
}) {
  const merkleTreeAccountPrior =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTree,
    );

  const merkleTreeUpdateState = getMerkleTreeUpdateStatePda(
    signer.publicKey,
    merkleTreeProgram,
  );
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
    console.error(
      "failed while initializing the merkle tree update state",
      err,
    );
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
  counter: { value: number };
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
  numberOfTransactions: number;
  signer: Keypair;
  merkleTreeUpdateState: PublicKey;
  transactionMerkleTree: PublicKey;
}) => {
  const transactions: Transaction[] = [];
  for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
    const transaction = new Transaction();
    transaction.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    );
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(counter.value))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    counter.value += 1;
    transaction.add(
      await merkleTreeProgram.methods
        .updateTransactionMerkleTree(new anchor.BN(counter.value))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTree,
        })
        .instruction(),
    );
    counter.value += 1;
    transactions.push(transaction);
  }
  return transactions;
};

const checkComputeInstructionsCompleted = async (
  merkleTreeProgram: Program<LightMerkleTreeProgram>,
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
    transactions.map(async (tx, i) => {
      try {
        await sendAndConfirmTransaction(
          connection,
          tx,
          [signer],
          confirmConfig,
        );
      } catch (err: any) {
        errors.push(err);
      }
    }),
  );

  if (errors.length > 0) throw errors[0];
};

/**
 * executeMerkleTreeUpdateTransactions attempts to execute a Merkle tree update.
 *
 * Strategy Overview:
 * - Sends an initial batch of 28 transactions including two compute instructions each.
 * - Checks if the update is complete.
 * - If not, sends additional batches of 10 transactions.
 * - This continues until the update is complete or a maximum retry limit of 240 instructions (120 transactions) is reached.
 *
 * @param {object} {
 *   merkleTreeProgram: Program<MerkleTreeProgram>,  // The Merkle tree program anchor instance.
 *   merkleTreeUpdateState: PublicKey,              // The public key of the temporary update state of the Merkle tree.
 *   transactionMerkleTree: PublicKey,              // The public key of the transaction Merkle tree which is updated.
 *   signer: Keypair,                               // The keypair used to sign the transactions.
 *   connection: Connection,                        // The network connection object.
 *   numberOfTransactions: number = 28,             // (optional) Initial number of transactions to send. Default is 28.
 *   interrupt: boolean = false                     // (optional) If true, interrupts the process. Default is false.
 * } - The input parameters for the function.
 *
 * @returns {Promise<void>} A promise that resolves when the update is complete or the maximum retry limit is reached.
 * @throws {Error} If an issue occurs while sending and confirming transactions.
 */
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
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
  merkleTreeUpdateState: PublicKey;
  transactionMerkleTree: PublicKey;
  signer: Keypair;
  connection: Connection;
  interrupt?: boolean;
}) {
  const counter = { value: 0 };
  let error = undefined;
  while (
    !(await checkComputeInstructionsCompleted(
      merkleTreeProgram,
      merkleTreeUpdateState,
    ))
  ) {
    numberOfTransactions = counter.value == 0 ? numberOfTransactions : 10;
    if (counter.value != 0) await sleep(1000);
    const transactions = await createTransactions({
      numberOfTransactions,
      signer,
      counter,
      merkleTreeProgram,
      merkleTreeUpdateState,
      transactionMerkleTree,
    });
    try {
      await sendAndConfirmTransactions(transactions, signer, connection);
    } catch (err: any) {
      error = err;
    }
    if (interrupt || counter.value >= 240) {
      console.log("Reached retry limit of 240 compute instructions");
      if (error) throw error;
      else return;
    }
  }
}
