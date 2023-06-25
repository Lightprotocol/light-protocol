import {
  BlockheightBasedTransactionConfirmationStrategy,
  Connection,
  Keypair,
  PublicKey,
  TransactionSignature,
} from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Relayer, RelayerSendTransactionsResponse } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { ConfirmOptions, Provider } from "../wallet";
import {
  indexRecentTransactions,
  sendVersionedTransaction,
  sendVersionedTransactions,
} from "../transaction";
import { IndexedTransaction } from "../types";
import { airdropSol } from "./airdrop";
import {
  TRANSACTION_MERKLE_TREE_KEY,
  IDL_MERKLE_TREE_PROGRAM,
  confirmConfig,
} from "../index";

export class TestRelayer extends Relayer {
  indexedTransactions: IndexedTransaction[] = [];
  relayerKeypair: Keypair;

  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipientSol?: PublicKey,
    relayerFee: BN = new BN(0),
    highRelayerFee?: BN,
    payer?: Keypair,
  ) {
    super(
      relayerPubkey,
      lookUpTable,
      relayerRecipientSol,
      relayerFee,
      highRelayerFee,
    );
    this.relayerKeypair = payer ? payer : Keypair.generate();
  }

  async updateMerkleTree(provider: Provider): Promise<any> {
    if (!provider.provider) throw new Error("Provider.provider is undefined.");
    if (!provider.url) throw new Error("Provider.provider is undefined.");

    const balance = await provider.provider.connection?.getBalance(
      this.relayerKeypair.publicKey,
    );

    if (!balance || balance < 1e9) {
      await airdropSol({
        provider: provider.provider,
        amount: 1_000_000_000,
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
    var res = await sendVersionedTransactions(instructions, provider);
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
