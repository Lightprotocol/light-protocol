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
  VersionedTransaction,
} from "@solana/web3.js";

import { Wallet } from "../wallet";
import { confirmConfig } from "../constants";
export const sendVersionedTransaction = async (
  ix: TransactionInstruction,
  connection: Connection,
  lookUpTable: PublicKey,
  payer: Wallet,
) => {
  const recentBlockhash = (await connection.getLatestBlockhash(confirmConfig))
    .blockhash;
  const ixSigner = ix.keys
    .map((key) => {
      if (key.isSigner) return key.pubkey;
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
    recentBlockhash: recentBlockhash,
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
};

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
  const signatures: TransactionSignature[] = [];
  for (const instruction of instructions) {
    const signature = await sendVersionedTransaction(
      instruction,
      connection,
      lookUpTable,
      payer,
    );
    if (!signature) {
      throw new Error("sendVersionedTransactions: signature is undefined");
    }
    signatures.push(signature);
    await confirmTransaction(connection, signature);
  }
  return { signatures };
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
