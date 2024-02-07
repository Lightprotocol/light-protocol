import { Program, AnchorProvider } from "@coral-xyz/anchor";
import { LightWasm } from "@lightprotocol/account.rs";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
import { Connection, PublicKey } from "@solana/web3.js";

import { Utxo } from "../utxo";

import {
  PublicTransactionIndexerEventBeet,
  fetchRecentPublicTransactions,
} from "../transaction";

import { SolMerkleTree, getRootIndex } from "../merkle-tree";

import { getVerifierProgramId } from "../transaction/psp-transaction";
import { RpcError, TransactionErrorCode } from "../errors";

import { merkleTreeProgramId, MERKLE_TREE_HEIGHT } from "../constants";
import {
  LightMerkleTreeProgram,
  AccountCompression,
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  IDL_PSP_ACCOUNT_COMPRESSION,
} from "../idls";
import { eventsToOutUtxos, outUtxosToUtxos } from "../utxo/parse-utxo";

type MerkleProof = string[];
export type MerkleProofsWithContext = {
  merkleProofs: MerkleProof[];
  root: string;
  index: number;
};

/// Exposes methods for interacting with the public test rpc
/// - getAssetsByOwner (returns all utxos owned by pubkey)
/// - getMerkleProofByIndexBatch
/// - getMerkleRoot
/// TODO:
/// - getCompressedAccount
/// - getCompressedAccountProof
/// - getCompressedProgramAccounts
/// - getCompressedTokenSupply
/// - getCompressedTokenAccountBalance (for mint, get balance + utxo + proof)
/// - getCompressedTokenAccountsByOwner (get all ctokens owned by pubkey)
/// - getCompressedTokenAccountsByDelegate
/// - return only active utxo state
/// TODO: make indexed transaction deserialization function generic at test rpc level
export class PublicTestRpc {
  indexedTransactions: PublicTransactionIndexerEventBeet[] = [];
  utxos: Utxo[] = [];
  connection: Connection;
  merkleTrees: SolMerkleTree[] = [];
  lightWasm: LightWasm;
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
  accountCompressionProgram: Program<AccountCompression>;
  latestSignature: string = "";
  merkleTreePublicKey: PublicKey;
  indexedArrayPublicKey: PublicKey;
  constructor({
    connection,
    lightWasm,
    merkleTreePublicKey,
    indexedArrayPublicKey,
  }: {
    merkleTreePublicKey: PublicKey;
    connection: Connection;
    lightWasm: LightWasm;
    indexedArrayPublicKey: PublicKey;
  }) {
    this.connection = connection;
    const solMerkleTree = new SolMerkleTree({
      lightWasm,
      pubkey: merkleTreePublicKey,
    });
    this.merkleTrees.push(solMerkleTree);
    this.lightWasm = lightWasm;
    this.merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      new AnchorProvider(connection, {} as any, {}),
    );
    this.accountCompressionProgram = new Program(
      IDL_PSP_ACCOUNT_COMPRESSION,
      getVerifierProgramId(IDL_PSP_ACCOUNT_COMPRESSION),
      new AnchorProvider(connection, {} as any, {}),
    );
    this.merkleTreePublicKey = merkleTreePublicKey;
    this.indexedArrayPublicKey = indexedArrayPublicKey;
  }

  /**
   * Indexes light transactions by:
   * - getting all signatures the merkle tree was involved in
   * - trying to extract and parse event cpi for every signature's transaction
   * Note that currently, this method ignores previously indexed transactions,
   * therefore it currently only supports indexing up to 1000 transactions.
   */
  async getIndexedTransactions(
    connection: Connection,
  ): Promise<PublicTransactionIndexerEventBeet[]> {
    const { transactions: newTransactions, oldestFetchedSignature } =
      await fetchRecentPublicTransactions({
        connection,
        batchOptions: {
          limit: 1000, /// This is also the default limit of signatures fetched in web3.js
        },
      });
    this.indexedTransactions = newTransactions;
    this.latestSignature = oldestFetchedSignature;

    const indexedOutUtxos = eventsToOutUtxos(
      this.indexedTransactions,
      this.lightWasm,
    );

    const merkleTree = new MerkleTree(
      MERKLE_TREE_HEIGHT,
      this.lightWasm,
      indexedOutUtxos.map(({ outUtxo }) => outUtxo.hash.toString()),
    );
    this.utxos = outUtxosToUtxos(indexedOutUtxos, this.lightWasm, merkleTree);
    return this.indexedTransactions;
  }

  /**
   * Returns the UTXOs of a given owner.
   */
  async getAssetsByOwner(owner: string): Promise<Utxo[]> {
    await this.getIndexedTransactions(this.connection);
    const ownedUtxos = this.utxos.filter(
      (utxo) => utxo.owner.toString() === owner,
    );

    // TODO: do not return nullified utxos, currently returns the full history of utxos
    // we need to deserialize the nullifier queue in ts first
    // let merkleTree = await this.syncMerkleTree(this.merkleTreePublicKey);
    // const indexedArrayAccount = await this.accountCompressionProgram.account.indexedArrayAccount.fetch(this.indexedArrayPublicKey);
    // const indexedArray = parseIndexedArrayFromAccount(Buffer.from(indexedArrayAccount.indexedArray));
    // // removes utxos which have been nullified
    // const spendableUtxos = merkleTree?.merkleTree.indexOf(utxo.hash.toString()) !== -1;
    return ownedUtxos;
  }

  /**
   * Synchronizes a Merkle tree with the current state of indexed transactions.
   * This method is intended for internal use to maintain the state of Merkle trees.
   *
   * @param {PublicKey} merkleTreePubkey - The public key of the Merkle tree to synchronize.
   * @returns {Promise<SolMerkleTree>} - The synchronized Merkle tree instance.
   *
   */
  async syncMerkleTree(merkleTreePubkey: PublicKey): Promise<SolMerkleTree> {
    let solMerkleTreeIndex = this.merkleTrees.findIndex((tree) =>
      tree.pubkey.equals(merkleTreePubkey),
    );
    solMerkleTreeIndex =
      solMerkleTreeIndex === -1 ? this.merkleTrees.length : solMerkleTreeIndex;

    const indexedOutUtxos = eventsToOutUtxos(
      this.indexedTransactions,
      this.lightWasm,
    );
    const merkleTree = new MerkleTree(
      MERKLE_TREE_HEIGHT,
      this.lightWasm,
      indexedOutUtxos.map(({ outUtxo }) => outUtxo.hash.toString()),
    );

    this.merkleTrees[solMerkleTreeIndex] = new SolMerkleTree({
      pubkey: merkleTreePubkey,
      lightWasm: this.lightWasm,
      merkleTree,
    });
    return this.merkleTrees[solMerkleTreeIndex];
  }

  /**
   * Returns the Merkle proofs for a batch of leaf indices in a Merkle tree
   *
   * @param {PublicKey} merkleTreePublicKey - The public key of the Merkle tree
   * @param {number[]} indices - The leaf indices to get Merkle proofs for
   * @returns - The Merkle proofs, root, and root index of the Merkle tree at time of evaluation
   */
  async getMerkleProofByIndexBatch(
    merkleTreePublicKey: PublicKey,
    indices: number[],
  ): Promise<MerkleProofsWithContext | undefined> {
    await this.getIndexedTransactions(this.connection);
    const merkleTree = await this.syncMerkleTree(merkleTreePublicKey);

    if (!merkleTree) return undefined;

    const rootIndex = await getRootIndex(
      this.accountCompressionProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
    );
    if (rootIndex === undefined) {
      throw new RpcError(
        TransactionErrorCode.ROOT_NOT_FOUND,
        "getRootIndex",
        `Root index not found for root ${merkleTree.merkleTree.root()}`,
      );
    }

    return {
      merkleProofs: indices.map(
        (index) => merkleTree.merkleTree.path(index).pathElements,
      ),
      root: merkleTree.merkleTree.root(),
      index: rootIndex.toNumber(),
    };
  }

  /**
   * Returns the current Merkle root of a Merkle tree
   *
   * @param {PublicKey} merkleTreePublicKey - The public key of the Merkle tree
   * @returns - The Merkle root and root index of the Merkle tree
   */
  async getMerkleRoot(
    merkleTreePublicKey: PublicKey,
  ): Promise<{ root: string; index: number } | undefined> {
    await this.getIndexedTransactions(this.connection);
    const merkleTree = await this.syncMerkleTree(merkleTreePublicKey);

    const rootIndex = await getRootIndex(
      this.accountCompressionProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
      "concurrentMerkleTreeAccount",
    );
    if (rootIndex === undefined) {
      throw new RpcError(
        TransactionErrorCode.ROOT_NOT_FOUND,
        "getRootIndex",
        `Root index not found for root ${merkleTree.merkleTree.root()}`,
      );
    }
    return { root: merkleTree.merkleTree.root(), index: rootIndex.toNumber() };
  }
}
