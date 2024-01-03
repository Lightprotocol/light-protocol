import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import {
  AnchorProvider,
  BN,
  BorshAccountsCoder,
  Program,
} from "@coral-xyz/anchor";
import { Relayer, RelayerSendTransactionsResponse } from "../relayer";
import { Provider, useWallet } from "../wallet";
import {
  fetchRecentTransactions,
  sendVersionedTransactions,
} from "../transaction";
import {
  ParsedIndexedTransaction,
  RelayerIndexedTransaction,
  RpcIndexedTransaction,
} from "../types";
import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  MerkleTreeConfig,
  BN_0,
  SolMerkleTree,
  UTXO_PREFIX_LENGTH,
  LightMerkleTreeProgram,
  RelayerError,
  TransactionErrorCode,
  merkleTreeProgramId,
} from "../index";
import { Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

export class TestRelayer extends Relayer {
  // @ts-ignore
  indexedTransactions: RelayerIndexedTransaction[] = [];
  relayerKeypair: Keypair;
  connection: Connection;
  merkleTrees: SolMerkleTree[] = [];
  hasher: Hasher;
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
  constructor({
    relayerPubkey,
    relayerRecipientSol,
    relayerFee = BN_0,
    highRelayerFee,
    payer,
    connection,
    hasher,
  }: {
    relayerPubkey: PublicKey;
    relayerRecipientSol?: PublicKey;
    relayerFee: BN;
    highRelayerFee?: BN;
    payer: Keypair;
    connection: Connection;
    hasher: Hasher;
  }) {
    super(relayerPubkey, relayerRecipientSol, relayerFee, highRelayerFee);
    if (payer.publicKey.toBase58() != relayerPubkey.toBase58())
      throw new Error(
        `Payer public key ${payer.publicKey.toBase58()} does not match relayer public key ${relayerPubkey.toBase58()}`,
      );
    this.relayerKeypair = payer;
    this.connection = connection;
    const solMerkleTree = new SolMerkleTree({
      hasher,
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });
    this.merkleTrees.push(solMerkleTree);
    this.hasher = hasher;
    this.merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      new AnchorProvider(connection, {} as any, {}),
    );
  }

  async sendTransactions(
    instructions: any[],
    provider: Provider,
  ): Promise<RelayerSendTransactionsResponse> {
    const res = await sendVersionedTransactions(
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
  // @ts-ignore
  async getIndexedTransactions(
    connection: Connection,
  ): Promise<RelayerIndexedTransaction[]> {
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
      this.indexedTransactions = newTransactions;
      return this.indexedTransactions;
    } else {
      if (this.indexedTransactions.length === 0) return [];

      const mostRecentTransaction = this.indexedTransactions.reduce((a, b) =>
        a.transaction.blockTime > b.transaction.blockTime ? a : b,
      );

      const { transactions: newTransactions } = await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit,
          until: mostRecentTransaction.transaction.signature,
        },
      });
      this.indexedTransactions = [
        ...this.indexedTransactions,
        ...newTransactions,
      ];
      return this.indexedTransactions;
    }
  }

  async syncMerkleTree(
    merkleTreePubkey: PublicKey,
    indexedTransactions: ParsedIndexedTransaction[],
  ): Promise<SolMerkleTree> {
    const solMerkleTreeIndex = this.merkleTrees.findIndex((tree) =>
      tree.pubkey.equals(merkleTreePubkey),
    );
    const rebuiltMt = await SolMerkleTree.build({
      pubkey: merkleTreePubkey,
      hasher: this.hasher,
      indexedTransactions,
    });
    this.merkleTrees[solMerkleTreeIndex] = rebuiltMt;
    return rebuiltMt;
  }

  async getEventById(
    merkleTreePdaPublicKey: PublicKey,
    id: string,
    variableNameID: number,
  ): Promise<RpcIndexedTransaction | undefined> {
    const indexedTransactions = await this.getIndexedTransactions(
      this.connection,
    );
    const indexedTransaction = indexedTransactions.find((trx) =>
      trx.IDs.includes(id),
    )?.transaction;
    if (!indexedTransaction) return undefined;
    const merkleTree = await this.syncMerkleTree(
      merkleTreePdaPublicKey,
      indexedTransactions.map((trx) => trx.transaction),
    );
    return createRpcIndexedTransaction(indexedTransaction, merkleTree);
  }

  async getEventsByIdBatch(
    merkleTreePdaPublicKey: PublicKey,
    ids: string[],
    variableNameID: number,
  ): Promise<RpcIndexedTransaction[] | undefined> {
    const indexedTransactions = await this.getIndexedTransactions(
      this.connection,
    );
    const indexedTransactionsById = indexedTransactions.filter((trx) =>
      trx.IDs.some((id) => ids.includes(id)),
    );
    const merkleTree = await this.syncMerkleTree(
      merkleTreePdaPublicKey,
      indexedTransactions.map((trx) => trx.transaction),
    );
    return indexedTransactionsById.map((trx) =>
      createRpcIndexedTransaction(trx.transaction, merkleTree),
    );
  }

  async getMerkleProofByIndexBatch(
    merkleTreePublicKey: PublicKey,
    indexes: number[],
  ): Promise<
    { merkleProofs: string[][]; root: string; index: number } | undefined
  > {
    const indexedTransactions = await this.getIndexedTransactions(
      this.connection,
    );
    const merkleTree = await this.syncMerkleTree(
      merkleTreePublicKey,
      indexedTransactions.map((trx) => trx.transaction),
    );
    if (!merkleTree) return undefined;
    const index = await getRootIndex(
      this.merkleTreeProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
    );

    return {
      merkleProofs: indexes.map(
        (index) => merkleTree.merkleTree.path(index).pathElements,
      ),
      root: merkleTree.merkleTree.root(),
      index: index.toNumber(),
    };
  }

  async getMerkleRoot(
    merkleTreePublicKey: PublicKey,
  ): Promise<{ root: string; index: number } | undefined> {
    const indexedTransactions = await this.getIndexedTransactions(
      this.connection,
    );
    const merkleTree = await this.syncMerkleTree(
      merkleTreePublicKey,
      indexedTransactions.map((trx) => trx.transaction),
    );
    const index = await getRootIndex(
      this.merkleTreeProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
    );
    return { root: merkleTree.merkleTree.root(), index: index.toNumber() };
  }
}

export async function getRootIndex(
  merkleTreeProgram: Program<LightMerkleTreeProgram>,
  merkleTreePublicKey: PublicKey,
  root: string,
) {
  const rootBytes = new BN(root).toArray("be", 32);
  const merkle_tree_account_data =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      merkleTreePublicKey,
      "confirmed",
    );
  let rootIndex: BN | undefined;
  // @ts-ignore: unknown type error
  merkle_tree_account_data.merkleTree.roots.map((x: any, index: any) => {
    if (x.toString() === rootBytes.toString()) {
      rootIndex = new BN(index.toString());
    }
  });

  if (rootIndex === undefined) {
    throw new RelayerError(
      TransactionErrorCode.ROOT_NOT_FOUND,
      "getRootIndex",
      `Root index not found for root${root}`,
    );
  }
  return rootIndex;
}

export const createRpcIndexedTransaction = (
  indexedTransaction: ParsedIndexedTransaction,
  merkleTree: SolMerkleTree,
): RpcIndexedTransaction => {
  const leavesIndexes = indexedTransaction.leaves.map((leaf) =>
    merkleTree.merkleTree.indexOf(new BN(leaf).toString()),
  );
  const merkleProofs = leavesIndexes.map(
    (index) => merkleTree.merkleTree.path(index).pathElements,
  );
  const rpcIndexedTransaction: RpcIndexedTransaction = {
    transaction: indexedTransaction,
    leavesIndexes,
    merkleProofs,
  };
  return rpcIndexedTransaction;
};

export const getIdsFromEncryptedUtxos = (
  encryptedUtxos: Buffer,
  numberOfLeaves: number,
): string[] => {
  const utxoLength = 124; //encryptedUtxos.length / numberOfLeaves;
  // divide encrypted utxos by multiples of 2
  // and extract the first two bytes of each
  const ids: string[] = [];
  for (let i = 0; i < encryptedUtxos.length; i += utxoLength) {
    ids.push(bs58.encode(encryptedUtxos.slice(i, i + UTXO_PREFIX_LENGTH)));
  }
  return ids;
};
