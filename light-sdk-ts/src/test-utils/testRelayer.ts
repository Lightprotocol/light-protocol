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
import { sendVersionedTransaction } from "./sendVersionedTransaction";
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

      const lookupTableAccount =
        await provider.provider.connection.getAccountInfo(
          provider.lookUpTable!,
          "confirmed",
        );

      const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
        lookupTableAccount!.data,
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

      compiledTx.addressTableLookups[0].accountKey = provider.lookUpTable!;

      const response = await sendVersionedTransaction(compiledTx, provider);
      return response;
    } catch (err) {
      console.error("erorr here =========>", { err });
      throw err;
    }
  }
}
