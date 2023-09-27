import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Relayer, RelayerSendTransactionsResponse } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider, useWallet } from "../wallet";
import {
  fetchRecentTransactions,
  sendVersionedTransactions,
} from "../transaction";
import { ParsedIndexedTransaction } from "../types";
import { airdropSol } from "./airdrop";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeConfig, BN_0 } from "../index";

export class TestRelayer extends Relayer {
  indexedTransactions: ParsedIndexedTransaction[] = [];
  relayerKeypair: Keypair;

  constructor({
    relayerPubkey,
    relayerRecipientSol,
    relayerFee = BN_0,
    highRelayerFee,
    payer,
  }: {
    relayerPubkey: PublicKey;
    relayerRecipientSol?: PublicKey;
    relayerFee: BN;
    highRelayerFee?: BN;
    payer: Keypair;
  }) {
    super(relayerPubkey, relayerRecipientSol, relayerFee, highRelayerFee);
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
        connection: provider.provider.connection!,
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
      provider.lookUpTables.versionedTransactionLookupTable,
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
  ): Promise<ParsedIndexedTransaction[]> {
    const merkleTreeAccountInfo = await connection.getAccountInfo(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
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
      let { transactions: newTransactions } = await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit,
        },
      });
      this.indexedTransactions = newTransactions.map((trx) => {
        return {
          ...trx,
          firstLeafIndex: new BN(trx.firstLeafIndex, "hex"),
          publicAmountSol: new BN(trx.publicAmountSol, "hex"),
          publicAmountSpl: new BN(trx.publicAmountSpl, "hex"),
          changeSolAmount: new BN(trx.changeSolAmount, "hex"),
          relayerFee: new BN(trx.relayerFee, "hex"),
        };
      });
      return this.indexedTransactions;
    } else {
      if (this.indexedTransactions.length === 0) return [];

      let mostRecentTransaction = this.indexedTransactions.reduce((a, b) =>
        a.blockTime > b.blockTime ? a : b,
      );

      let { transactions: newTransactions } = await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit,
          until: mostRecentTransaction.signature,
        },
      });
      let parsedNewTransactions = newTransactions.map((trx) => {
        return {
          ...trx,
          firstLeafIndex: new BN(trx.firstLeafIndex, "hex"),
          publicAmountSol: new BN(trx.publicAmountSol, "hex"),
          publicAmountSpl: new BN(trx.publicAmountSpl, "hex"),
          changeSolAmount: new BN(trx.changeSolAmount, "hex"),
          relayerFee: new BN(trx.relayerFee, "hex"),
        };
      });
      this.indexedTransactions = [
        ...this.indexedTransactions,
        ...parsedNewTransactions,
      ];
      return this.indexedTransactions;
    }
  }
}
