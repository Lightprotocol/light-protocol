import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import {
  indexRecentTransactions,
  sendVersionedTransaction,
} from "../transaction";
import { IndexedTransaction } from "../types";
import { airdropSol } from "./airdrop";
import { TRANSACTION_MERKLE_TREE_KEY, IDL_MERKLE_TREE_PROGRAM } from "../index";

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
      console.error("erorr here =========>", { err });
      throw err;
    }
  }

  async getIndexedTransactions(
    connection: Connection,
  ): Promise<IndexedTransaction[]> {
    const merkleTreeAccountInfo = await connection.getAccountInfo(
      TRANSACTION_MERKLE_TREE_KEY,
      "confirmed",
    );
    const coder = new BorshAccountsCoder(IDL_MERKLE_TREE_PROGRAM);
    if (!merkleTreeAccountInfo)
      throw new Error("Failed to fetch merkle tree account");
    const merkleTreeAccount = coder.decode(
      "transactionMerkleTree",
      merkleTreeAccountInfo.data,
    );
    const merkleTreeIndex = merkleTreeAccount.nextIndex;

    const limit = 1000 + 260 * merkleTreeIndex.toNumber();
    if (this.indexedTransactions.length === 0) {
      this.indexedTransactions = await indexRecentTransactions({
        connection,
        batchOptions: {
          limit,
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
          limit,
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
