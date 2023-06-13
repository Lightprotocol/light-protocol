import {
  AddressLookupTableAccount,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  TransactionInstruction,
  TransactionMessage,
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

// currently not used
export const sendTransactionWithConnection = async (
  instructions: [TransactionInstruction],
  connection: Connection,
  signer: Keypair,
) => {
  const recentBlockhash = (await connection.getLatestBlockhash(confirmConfig))
    .blockhash;

  const txMsg = new TransactionMessage({
    payerKey: signer.publicKey,
    instructions,
    recentBlockhash,
  });
  const v0Message = txMsg.compileToV0Message();

  var tx = new VersionedTransaction(v0Message);
  tx.sign([signer]);
  let retries = 3;
  let res;
  while (retries > 0) {
    try {
      res = await connection.sendTransaction(tx, confirmConfig);
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
