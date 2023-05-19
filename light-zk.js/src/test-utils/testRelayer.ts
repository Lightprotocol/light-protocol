import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import {
  indexRecentTransactions,
  sendVersionedTransaction,
} from "../transaction";
import { IndexedTransaction } from "../types";
import { airdropSol } from "./airdrop";
export class TestRelayer extends Relayer {
  indexedTransactions: IndexedTransaction[] = [];
  relayerKeypair: Keypair;

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
    this.relayerKeypair = Keypair.generate();
  }

  async updateMerkleTree(provider: Provider): Promise<any> {
    if (!provider.provider) throw new Error("Provider.provider is undefined.");
    await airdropSol({
      provider: provider.provider,
      amount: 1_000_000_000,
      recipientPublicKey: this.relayerKeypair.publicKey,
    });
    try {
      const response = await updateMerkleTreeForTest(
        this.relayerKeypair,
        provider.provider,
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
      console.log({ err });
      throw err;
    }
  }

  async getIndexedTransactions(
    connection: Connection,
  ): Promise<IndexedTransaction[]> {
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
      ].sort(
        (a, b) => a.firstLeafIndex.toNumber() - b.firstLeafIndex.toNumber(),
      );
      return this.indexedTransactions;
    }
  }
}
