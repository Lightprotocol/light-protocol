import { Connection, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import {
  indexRecentTransactions,
  sendVersionedTransaction,
} from "../transaction";
import { indexedTransaction } from "types";
export class TestRelayer extends Relayer {
  indexedTransactions: indexedTransaction[] = [];

  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipientSol?: PublicKey,
    relayerFee: BN = new BN(0),
    highRelayerFee?: BN,
  ) {
    super(
      relayerPubkey,
      lookUpTable,
      relayerRecipientSol,
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
      const response = await sendVersionedTransaction(instruction, provider);
      return response;
    } catch (err) {
      console.error("erorr here =========>", { err });
      throw err;
    }
  }

  async getIndexedTransactions(
    connection: Connection,
  ): Promise<indexedTransaction[]> {
    if (this.indexedTransactions.length === 0) {
      this.indexedTransactions = await indexRecentTransactions({
        connection,
        batchOptions: {
          limit: 5000,
        },
        dedupe: false,
      });

      return this.indexedTransactions;
    } else {
      if (this.indexedTransactions.length === 0) return [];
      let mostRecentTransaction = this.indexedTransactions.reduce((a, b) =>
        a.blockTime > b.blockTime ? a : b,
      );

      let newTransactions = await indexRecentTransactions({
        connection,
        batchOptions: {
          limit: 5000,
          until: mostRecentTransaction.signature,
        },
        dedupe: false,
      });
      this.indexedTransactions = [
        ...newTransactions,
        ...this.indexedTransactions,
      ];
      return this.indexedTransactions;
    }
  }
}
