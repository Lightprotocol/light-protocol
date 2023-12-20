import {
  Commitment,
  ComputeBudgetProgram,
  Connection,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  TransactionMessage,
  TransactionSignature,
  Blockhash,
  VersionedTransaction,
  AddressLookupTableAccountArgs,
  BlockhashWithExpiryBlockHeight,
  PublicKey,
} from "@solana/web3.js";

import { PrioritizationFee, SignaturesWithBlockhashInfo } from "../types";

/**
 * Creates a Light Transaction (array of VersionedTransactions) from a given array of TransactionInstructions.
 * Txs must be executed in the order they appear in the array.
 * The last tx verifies the correctness of the state transition and updates the protocol state.
 * All preceding txs (if any) send data to the chain.
 */
export async function createSolanaTransactions(
  ixs: TransactionInstruction[],
  recentBlockhash: Blockhash,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  prioritizationFee?: PrioritizationFee,
): Promise<VersionedTransaction[]> {
  const versionedTransactions: VersionedTransaction[] = [];

  for (const ix of ixs) {
    versionedTransactions.push(
      await createVersionedTransaction(
        ix,
        versionedTransactionLookupTableAccountArgs,
        recentBlockhash,
        prioritizationFee,
      ),
    );
  }
  return versionedTransactions;
}

/**
 * Creates and compiles a VersionedTransaction from a TransactionInstruction.
 */
export async function createVersionedTransaction(
  ix: TransactionInstruction,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  recentBlockhash: Blockhash,
  prioritizationFee?: PrioritizationFee,
): Promise<VersionedTransaction> {
  const payerKey = ix.keys.find((key) => key.isSigner)?.pubkey;

  if (!payerKey) {
    throw new Error(`Instruction must have one signer`);
  }

  // TODO: we should set cu to minimum required for execution to save cost and maximize throughput.
  const txMsg = new TransactionMessage({
    payerKey,
    instructions: prioritizationFee
      ? [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ComputeBudgetProgram.setComputeUnitPrice({
            microLamports: prioritizationFee,
          }),
          ix,
        ]
      : [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }), ix],
    recentBlockhash,
  });
  // console.log("txMsg", JSON.stringify(txMsg));

  const { state, key } = versionedTransactionLookupTableAccountArgs;
  // const conn = new Connection("http://127.0.0.1:8899");

  // const acc = (
  //   await conn.getAddressLookupTable(
  //     new PublicKey("EUwVvyTZrBVyB1LUQf6b6WWcdXgcgYz5E5mcWUtkW2vX"),
  //   )
  // ).value;
  // console.log("acc", acc);

  const compiledTx = txMsg.compileToV0Message([
    // acc!,
    {
      state,
      key,
      isActive: () => true,
    },
  ]);
  // if (compiledTx.addressTableLookups[0]) {
  //   compiledTx.addressTableLookups[0].accountKey = key;
  // }
  // console.log("compiledTx", JSON.stringify(compiledTx));

  const versionedTransaction = new VersionedTransaction(compiledTx);
  return versionedTransaction;
}

export type SendVersionedTransactionsResult = {
  signatures?: TransactionSignature[];
  error?: any;
};

export async function confirmTransaction(
  connection: Connection,
  signature: TransactionSignature,
  confirmation: Commitment = "confirmed",
  blockhashInfo?: BlockhashWithExpiryBlockHeight,
) {
  const latestBlockHashInfo: BlockhashWithExpiryBlockHeight =
    blockhashInfo || (await connection.getLatestBlockhash(confirmation));

  const strategy: TransactionConfirmationStrategy = {
    signature: signature.toString(),
    lastValidBlockHeight: latestBlockHashInfo.lastValidBlockHeight,
    blockhash: latestBlockHashInfo.blockhash,
  };
  return await connection.confirmTransaction(strategy, confirmation);
}
