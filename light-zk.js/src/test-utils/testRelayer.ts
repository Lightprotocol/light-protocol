import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Relayer, RelayerSendTransactionsResponse } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider, useWallet } from "../wallet";
import {
  indexRecentTransactions,
  sendVersionedTransactions,
} from "../transaction";
import { IndexedTransaction } from "../types";
import { airdropSol } from "./airdrop";
import { TRANSACTION_MERKLE_TREE_KEY, IDL_MERKLE_TREE_PROGRAM } from "../index";

export class TestRelayer extends Relayer {
  indexedTransactions: IndexedTransaction[] = [];
  relayerKeypair: Keypair;

  constructor({
    relayerPubkey,
    lookUpTable,
    relayerRecipientSol,
    relayerFee = new BN(0),
    highRelayerFee,
    payer,
  }: {
    relayerPubkey: PublicKey;
    lookUpTable: PublicKey;
    relayerRecipientSol?: PublicKey;
    relayerFee: BN;
    highRelayerFee?: BN;
    payer: Keypair;
  }) {
    super(
      relayerPubkey,
      lookUpTable,
      relayerRecipientSol,
      relayerFee,
      highRelayerFee,
    );
    if (payer.publicKey.toBase58() != relayerPubkey.toBase58())
      throw new Error(
        `Payer public key ${payer.publicKey.toBase58()} does not match relayer public key ${relayerPubkey.toBase58()}`,
      );
    this.relayerKeypair = payer;
  }

  async updateMerkleTree(provider: Provider): Promise<any> {
    if (!provider.provider) throw new Error("Provider.provider is undefined.");
    if (!provider.url) throw new Error("Provider.provider is undefined.");
    if (provider.url !== "http://127.0.0.1:8899")
      throw new Error("Provider url is not http://127.0.0.1:8899");

    const balance = await provider.provider.connection?.getBalance(
      this.relayerKeypair.publicKey,
    );

    if (!balance || balance < 1e9) {
      await airdropSol({
        provider: provider.provider,
        lamports: 1_000_000_000,
        recipientPublicKey: this.relayerKeypair.publicKey,
      });
    }

    try {
      const response = await updateMerkleTreeForTest(
        this.relayerKeypair,
        provider.url,
      );
      return response;
    } catch (e) {
      console.log(e);
      throw e;
    }
  }

  async sendTransactions(
    instructions: any[],
    provider: Provider,
  ): Promise<RelayerSendTransactionsResponse> {
    var res = await sendVersionedTransactions(
      instructions,
      provider.provider!.connection!,
      this.accounts.lookUpTable,
      useWallet(this.relayerKeypair),
    );
    if (res.error) return { transactionStatus: "error", ...res };
    else return { transactionStatus: "confirmed", ...res };
  }

  /**
   * Indexes light transactions by:
   * - getting all signatures the merkle tree was involved in
   * - trying to extract and parse event cpi for every signature's transaction
   * - if there are indexed transactions already in the relayer object only transactions after the last indexed event are indexed
   * @param connection
   * @returns
   */
  async getIndexedTransactions(
    connection: Connection,
  ): Promise<IndexedTransaction[]> {
    const merkleTreeAccountInfo = await connection.getAccountInfo(
      TRANSACTION_MERKLE_TREE_KEY,
      "confirmed",
    );
    if (!merkleTreeAccountInfo)
      throw new Error("Failed to fetch merkle tree account");
    const coder = new BorshAccountsCoder(IDL_MERKLE_TREE_PROGRAM);
    const merkleTreeAccount = coder.decode(
      "transactionMerkleTree",
      merkleTreeAccountInfo.data,
    );

    // limits the number of signatures which are queried
    // if the number is too low it is not going to index all transactions
    // hence the dependency on the merkle tree account index times 260 transactions
    // which is approximately the number of transactions sent to send one shielded transaction and update the merkle tree
    const limit = 1000 + 260 * merkleTreeAccount.nextIndex.toNumber();
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

      await indexRecentTransactions({
        connection,
        batchOptions: {
          limit,
          until: mostRecentTransaction.signature,
        },
        dedupe: false,
        transactions: this.indexedTransactions,
      });
      return this.indexedTransactions;
    }
  }
}
