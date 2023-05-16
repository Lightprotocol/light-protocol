import {
  AddressLookupTableAccount,
  ComputeBudgetProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";

import { Provider } from "../wallet";
import { confirmConfig } from "../constants";

/**
 * This function sends a versioned transaction to the Solana blockchain.
 *
 * It first fetches the recent blockhash and creates a `TransactionMessage`, which includes
 * instructions to set the compute unit limit and the instruction passed to the function.
 *
 * Then, it fetches the lookup table account data and deserializes it. This data is used
 * to compile the transaction message.
 *
 * It then creates a new versioned transaction and tries to sign and send it to the
 * Solana blockchain. If any error occurs, it retries up to three times before logging
 * the error and returning it.
 *
 * @param ix - The instruction to be included in the transaction.
 * @param provider - An object that contains a Solana wallet and network connection.
 * @returns A promise that resolves to the result of sending the raw transaction or
 *          an error if the transaction fails to be sent after three attempts.
 * @throws Will log any errors that occur while signing or sending the transaction.
 */
export const sendVersionedTransaction = async (ix: any, provider: Provider) => {
  const recentBlockhash = (
    await provider.provider!.connection.getRecentBlockhash("confirmed")
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
      let serializedTx = tx.serialize();

      res = await provider.provider!.connection.sendRawTransaction(
        serializedTx,
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
