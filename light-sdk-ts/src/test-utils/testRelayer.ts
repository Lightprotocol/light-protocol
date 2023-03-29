import {
  PublicKey,
  Connection,
  TransactionMessage,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  VersionedTransaction,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import { confirmConfig } from "../constants";

export class TestRelayer extends Relayer {
  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient?: PublicKey,
    relayerFee: BN = new BN(0),
    highRelayerFee?: BN,
  ) {
    super(
      relayerPubkey,
      lookUpTable,
      relayerRecipient,
      relayerFee,
      highRelayerFee,
    );
  }

  async updateMerkleTree(provider: Provider): Promise<any> {
    try {
      const response = await updateMerkleTreeForTest(
        provider.provider?.connection!,
      );
      return response;
    } catch (e) {
      console.log(e);
      throw e;
    }
  }

  async sendTransaction(instruction: any, provider: Provider): Promise<any> {
    try {
      if (!provider.provider) throw new Error("no provider set");

      const recentBlockhash = (
        await provider.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;

      console.log({ recentBlockhash });

      const txMsg = new TransactionMessage({
        payerKey: provider.relayer.accounts.relayerPubkey,
        instructions: [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          instruction,
        ],
        recentBlockhash: recentBlockhash,
      });

      console.log({ txMsg });

      const lookupTableAccount =
        await provider.provider.connection.getAccountInfo(
          provider.lookUpTable!,
          "confirmed",
        );

      console.log({ lookupTableAccount });

      const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
        lookupTableAccount!.data,
      );

      console.log({ unpackedLookupTableAccount });

      console.log(
        "loookup table here =============>",
        provider.lookUpTable!,
        provider.relayer.accounts.lookUpTable!,
      );

      const compiledTx = txMsg.compileToV0Message([
        {
          state: unpackedLookupTableAccount,
          key: provider.relayer.accounts.lookUpTable!,
          isActive: () => {
            return true;
          },
        },
      ]);

      console.log({ compiledTx });

      compiledTx.addressTableLookups[0].accountKey = provider.lookUpTable!;
      let tx = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res;
      while (retries > 0) {
        tx = await provider.wallet.signTransaction(tx);
        try {
          let serializedTx = tx.serialize();
          console.log("tx: ", serializedTx);
          res = await provider.provider.connection.sendRawTransaction(
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
    } catch (err) {
      console.error("erorr here =========>", { err });
      throw err;
    }
  }
}
