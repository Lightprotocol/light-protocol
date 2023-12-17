import {
  AddressLookupTableAccount,
  Commitment,
  ComputeBudgetProgram,
  Connection,
  PublicKey,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  TransactionMessage,
  TransactionSignature,
  Blockhash,
  VersionedTransaction,
  AddressLookupTableAccountArgs,
  ConfirmOptions,
  RecentPrioritizationFees,
} from "@solana/web3.js";

import { Provider, Wallet } from "../wallet";
import { confirmConfig } from "../constants";
import { Action } from "./transaction";
import { PrioritizationFee } from "../types";

/**
 * Creates a Light Transaction (array of VersionedTransactions) from a given array of TransactionInstructions.
 * Txs must be executed in the order they appear in the array.
 * The last tx verifies the correctness of the state transition and updates the protocol state.
 * All preceding txs (if any) send data to the chain.
 * @param ixs
 * @param recentBlockhash
 * @param versionedTransactionLookupTableAccountArgs
 * @returns LightTransaction - VersionedTransaction[]
 */
export function createSolanaTransactions(
  ixs: TransactionInstruction[],
  recentBlockhash: Blockhash,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  prioritizationFee?: PrioritizationFee,
): VersionedTransaction[] {
  const versionedTransactions: VersionedTransaction[] = [];

  for (const ix of ixs) {
    versionedTransactions.push(
      createVersionedTransaction(
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
 * @param ix
 * @param versionedTransactionLookupTableAccountArgs
 * @param recentBlockhash
 * @returns VersionedTransaction
 */
export function createVersionedTransaction(
  ix: TransactionInstruction,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  recentBlockhash: Blockhash,
  prioritizationFee?: PrioritizationFee,
): VersionedTransaction {
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

  const { state, key } = versionedTransactionLookupTableAccountArgs;

  const compiledTx = txMsg.compileToV0Message([
    {
      state,
      key,
      isActive: () => true,
    },
  ]);
  if (compiledTx.addressTableLookups[0]) {
    compiledTx.addressTableLookups[0].accountKey = key;
  }

  const versionedTransaction = new VersionedTransaction(compiledTx);
  return versionedTransaction;
}

export async function sendVersionedTransaction(
  ix: TransactionInstruction,
  connection: Connection,
  lookUpTable: PublicKey,
  versionedTransactionLookupTableAccountArgs: AddressLookupTableAccountArgs,
  payer: Wallet,
  recentBlockhash: Blockhash,
) {
  const ixSigner = ix.keys
    .map((key) => {
      if (key.isSigner == true) return key.pubkey;
    })[0]
    ?.toBase58();
  if (payer.publicKey.toBase58() != ixSigner) {
    throw new Error(
      ` payer pubkey is not equal instruction signer ${payer.publicKey.toBase58()} != ${ixSigner} (only one signer supported)`,
    );
  }
  const txMsg = new TransactionMessage({
    payerKey: payer.publicKey,
    instructions: [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ix,
    ],
    recentBlockhash,
  });

  const lookupTableAccount = await connection.getAccountInfo(
    lookUpTable,
    "confirmed",
  );

  const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
    lookupTableAccount!.data,
  );

  const compiledTx = txMsg.compileToV0Message([
    {
      state: unpackedLookupTableAccount,
      key: lookUpTable,
      isActive: () => {
        return true;
      },
    },
  ]);
  if (compiledTx.addressTableLookups[0]) {
    compiledTx.addressTableLookups[0].accountKey = lookUpTable;
  }

  let tx = new VersionedTransaction(compiledTx);
  let retries = 3;
  while (retries > 0) {
    tx = await payer.signTransaction(tx);
    try {
      return await connection.sendTransaction(tx, confirmConfig);
    } catch (e: any) {
      console.log(e);
      retries--;
      if (retries == 0 || e.logs !== undefined) {
        console.log(e);
        throw e;
      }
    }
  }
}

export type SendVersionedTransactionsResult = {
  signatures?: TransactionSignature[];
  error?: any;
};

export async function sendVersionedTransactions(
  instructions: any[],
  connection: Connection,
  lookUpTable: PublicKey,
  payer: Wallet,
): Promise<SendVersionedTransactionsResult> {
  try {
    const signatures: TransactionSignature[] = [];
    for (const instruction of instructions) {
      const signature = await sendVersionedTransaction(
        instruction,
        connection,
        lookUpTable,
        payer,
      );
      if (!signature)
        throw new Error("sendVersionedTransactions: signature is undefined");
      signatures.push(signature);
      await confirmTransaction(connection, signature);
    }
    return { signatures };
  } catch (error) {
    return { error };
  }
}

export async function confirmTransaction(
  connection: Connection,
  signature: string,
  confirmation: Commitment = "confirmed",
) {
  const latestBlockHash = await connection.getLatestBlockhash(confirmation);
  const strategy: TransactionConfirmationStrategy = {
    signature: signature.toString(),
    lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    blockhash: latestBlockHash.blockhash,
  };
  return await connection.confirmTransaction(strategy, confirmation);
}
