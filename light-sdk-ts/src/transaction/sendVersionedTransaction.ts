import {
  AddressLookupTableAccount,
  ComputeBudgetProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
<<<<<<<< HEAD:light-sdk-ts/src/transaction/sendVersionedTransaction.ts
import { Provider } from "wallet";
import { confirmConfig } from "../constants";
========
import { ADMIN_AUTH_KEYPAIR, confirmConfig, Provider } from "light-sdk";

export async function sendTransaction(
  ix: any,
): Promise<TransactionSignature | undefined> {
  const provider = await Provider.init({ wallet: ADMIN_AUTH_KEYPAIR });
  if (!provider.provider) throw new Error("no provider set");
>>>>>>>> a1dc3a0b (refactor the relayer):relayer/src/services/transactionService.ts

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

<<<<<<<< HEAD:light-sdk-ts/src/transaction/sendVersionedTransaction.ts
  const lookupTableAccount = await provider.provider!.connection.getAccountInfo(
    provider.relayer.accounts.lookUpTable,
========
  const lookupTableAccount = await provider.provider.connection.getAccountInfo(
    provider.lookUpTable!,
>>>>>>>> a1dc3a0b (refactor the relayer):relayer/src/services/transactionService.ts
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

  compiledTx.addressTableLookups[0].accountKey =
    provider.relayer.accounts.lookUpTable;

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
