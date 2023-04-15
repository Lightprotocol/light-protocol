import { Connection, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import {
  getRecentTransactions,
  sendVersionedTransaction,
} from "../transaction";
import { indexedTransaction } from "types";
export class TestRelayer extends Relayer {
  transactionHistory: indexedTransaction[] = [];

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
    if (this.transactionHistory.length === 0) {
      let olderTransactions = await getRecentTransactions({
        connection,
        batchOptions: {
          limit: 5000,
        },
        dedupe: false,
      });

      this.transactionHistory = olderTransactions;

      return this.transactionHistory;
    } else {
      if (this.transactionHistory.length === 0) return [];
      let mostRecentTransaction = this.transactionHistory.reduce((a, b) =>
        a.blockTime > b.blockTime ? a : b,
      );

      let newerTransactions = await getRecentTransactions({
        connection,
        batchOptions: {
          limit: 5000,
          until: mostRecentTransaction.signature,
        },
        dedupe: false,
      });
      this.transactionHistory = [
        ...newerTransactions,
        ...this.transactionHistory,
      ];
      return this.transactionHistory;
    }
  }
}
