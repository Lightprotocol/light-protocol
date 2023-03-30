import { VersionedTransaction } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Provider } from "wallet";
import { confirmConfig } from "../constants";

export const sendVersionedTransaction = async (
  compiledTx: anchor.web3.MessageV0,
  provider: Provider,
) => {
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
