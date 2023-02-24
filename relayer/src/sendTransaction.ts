import {
  TransactionSignature,
  TransactionMessage,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  VersionedTransaction,
} from "@solana/web3.js";
import { confirmConfig, Provider } from "light-sdk";

export async function sendTransaction(
  ix: any,
  provider: Provider
): Promise<TransactionSignature | undefined> {
  if (!provider.provider) throw new Error("no provider set");

  const recentBlockhash = (
    await provider.provider.connection.getRecentBlockhash("confirmed")
  ).blockhash;
  const txMsg = new TransactionMessage({
    payerKey: provider.nodeWallet!.publicKey,
    instructions: [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ix,
    ],
    recentBlockhash: recentBlockhash,
  });

  const lookupTableAccount = await provider.provider.connection.getAccountInfo(
    provider.lookUpTable!,
    "confirmed"
  );

  const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
    lookupTableAccount!.data
  );

  const compiledTx = txMsg.compileToV0Message([
    {
      state: unpackedLookupTableAccount,
      key: provider.lookUpTable!,
      isActive: () => {
        return true;
      },
    },
  ]);

  compiledTx.addressTableLookups[0].accountKey = provider.lookUpTable!;

  var tx = new VersionedTransaction(compiledTx);
  let retries = 3;
  let res;
  while (retries > 0) {
    tx.sign([provider.nodeWallet!]);

    try {
      let serializedTx = tx.serialize();
      console.log("tx: ");

      res = await provider.provider.connection.sendRawTransaction(
        serializedTx,
        confirmConfig
      );
      retries = 0;
      // console.log(res);
    } catch (e: any) {
      retries--;
      if (retries == 0 || e.logs !== undefined) {
        console.log(e);
        return e;
      }
    }
  }
  return res;
}
