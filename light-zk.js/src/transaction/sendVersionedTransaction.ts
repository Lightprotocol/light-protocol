import {
  AddressLookupTableAccount,
  BlockheightBasedTransactionConfirmationStrategy,
  ComputeBudgetProgram,
  ConfirmOptions,
  Connection,
  Keypair,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  TransactionMessage,
  TransactionSignature,
  VersionedTransaction,
} from "@solana/web3.js";

import { Provider } from "../wallet";
import { confirmConfig } from "../constants";
export const sendVersionedTransaction = async (ix: any, provider: Provider) => {
  const recentBlockhash = (
    await provider.provider!.connection.getLatestBlockhash(confirmConfig)
  ).blockhash;

  const txMsg = new TransactionMessage({
    payerKey: provider.relayer.accounts.relayerPubkey,
    instructions: [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ix,
    ],
    recentBlockhash: recentBlockhash,
  });

  const lookupTableAccount = await provider.provider!.connection.getAccountInfo(
    provider.relayer.accounts.lookUpTable,
    "confirmed",
  );

  const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
    lookupTableAccount!.data,
  );

  const compiledTx = txMsg.compileToV0Message([
    {
      state: unpackedLookupTableAccount,
      key: provider.relayer.accounts.lookUpTable,
      isActive: () => {
        return true;
      },
    },
  ]);
  if (compiledTx.addressTableLookups[0]) {
    compiledTx.addressTableLookups[0].accountKey =
      provider.relayer.accounts.lookUpTable;
  }

  var tx = new VersionedTransaction(compiledTx);
  let retries = 3;
  let res;
  while (retries > 0) {
    tx = await provider.wallet.signTransaction(tx);
    try {
      res = await provider.provider!.connection.sendTransaction(
        tx,
        confirmConfig,
      );
      retries = 0;
    } catch (e: any) {
      retries--;
      if (retries == 0 || e.logs !== undefined) {
        console.log(e);
        return e;
      }
    }
  }
  return res;
};

export type SendVersionedTransactionsResult = {
  signatures?: TransactionSignature[];
  error?: any;
};

export async function sendVersionedTransactions(
  instructions: any[],
  provider: Provider,
): Promise<SendVersionedTransactionsResult> {
  let signature;
  try {
    if (!provider.provider) throw new Error("no provider set");
    var signatures: TransactionSignature[] = [];
    for (var instruction of instructions) {
      signature = await sendVersionedTransaction(instruction, provider);
      signatures.push(signature);
      const latestBlockHash =
        await provider.provider!.connection!.getLatestBlockhash("confirmed");
      let strategy: TransactionConfirmationStrategy = {
        signature: signature.toString(),
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        blockhash: latestBlockHash.blockhash,
      };
      await provider.provider.connection?.confirmTransaction(strategy);
    }
    return { signatures };
  } catch (error) {
    return { error: signature };
  }
}
