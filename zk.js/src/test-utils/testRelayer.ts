import {
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { Provider } from "../wallet";
import { fetchRecentTransactions } from "../transaction";
import {
  ParsedIndexedTransaction,
  PrioritizationFee,
  SignaturesWithBlockhashInfo,
} from "../types";
import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  MerkleTreeConfig,
  BN_0,
} from "../index";

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

  /**
   * Convenience function for sending and confirming instructions via Light RPC node.
   * Routes instructions to Light RPC node and confirms the last transaction signature.
   */
  async sendAndConfirmSolanaInstructions(
    ixs: TransactionInstruction[],
    connection: Connection,
    confirmOptions?: ConfirmOptions,
    prioritizationFee?: PrioritizationFee,
    provider?: Provider,
  ): Promise<TransactionSignature[]> {
    console.log("@testRelayer.sendAndConfirmSolanaInstructions");

    const {
      signatures,
      blockhashInfo: { lastValidBlockHeight, blockhash },
    } = await this.sendSolanaInstructions(ixs, prioritizationFee, provider!);

    const lastTxIndex = signatures.length - 1;

    const strategy: TransactionConfirmationStrategy = {
      signature: signatures[lastTxIndex],
      lastValidBlockHeight,
      blockhash,
    };
    await connection.confirmTransaction(strategy, confirmOptions?.commitment);

    return signatures;
  }
  /**
   * Mocks sending a transaction to the relayer, executes by itself
   * Contrary to the actual relayer response, this mock has already
   * confirmed the transaction by the time it returns
   */
  async sendSolanaInstructions(
    ixs: TransactionInstruction[],
    prioritizationFee?: bigint,
    provider?: Provider,
  ): Promise<SignaturesWithBlockhashInfo> {
    // we're passing the blockhashinfo manually to be able to mock the return type of the 'sendSolanaInstructions' Relayer method
    const blockhashInfo =
      await provider!.connection!.getLatestBlockhash("finalized"); // must get finalized blockhash to avoid "BlockhashNotFound" error
    console.log("blockhashInfo", blockhashInfo);
    /// We're confirming with finalized just in this test case. TODO: check whether workaround
    const signatures = await provider!.sendAndConfirmSolanaInstructions(
      ixs,
      { commitment: "finalized" },
      prioritizationFee,
      blockhashInfo,
    );

    return { signatures, blockhashInfo };
  }

  /**
   * Indexes light transactions by:
   * - getting all signatures the merkle tree was involved in
   * - trying to extract and parse event cpi for every signature's transaction
   * - if there are indexed transactions already in the relayer object only transactions after the last indexed event are indexed
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
    const coder = new BorshAccountsCoder(IDL_LIGHT_MERKLE_TREE_PROGRAM);
    const merkleTreeAccount = coder.decode(
      "transactionMerkleTree",
      merkleTreeAccountInfo.data,
    );

    // limits the number of signatures which are queried
    // if the number is too low it is not going to index all transactions
    // hence the dependency on the merkle tree account index times 260 transactions
    // which is approximately the number of transactions sent to send one shielded transaction and update the merkle tree
    const limit =
      1000 + 260 * merkleTreeAccount.merkleTree.nextIndex.toNumber();
    if (this.indexedTransactions.length === 0) {
      const { transactions: newTransactions } = await fetchRecentTransactions({
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

      const mostRecentTransaction = this.indexedTransactions.reduce((a, b) =>
        a.blockTime > b.blockTime ? a : b,
      );

      const { transactions: newTransactions } = await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit,
          until: mostRecentTransaction.signature,
        },
      });
      const parsedNewTransactions = newTransactions.map((trx) => {
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
