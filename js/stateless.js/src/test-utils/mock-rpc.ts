import {
  ConfirmOptions,
  Connection,
  Keypair,
  PublicKey,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";

import { LightWasm } from "@lightprotocol/account.rs";

import { fetchRecentPublicTransactions } from "./index-transaction";

import { merkleTreeProgramId } from "../constants";
import { IDL } from "../idls/account_compression";
import { GetUtxoConfig, WithMerkleUpdateContext } from "../rpc-interface";
import { UtxoWithMerkleContext, bigint254 } from "../state";

export function getMockRpc(connection: Connection) {
  return new MockRpc({
    connection,
  });
}

export class MockRpc {
  indexedTransactions: any[] = [];
  connection: Connection;
  // merkleTrees: SolMerkleTree[] = [];
  // lightWasm: LightWasm;
  // accountCompressionProgram: Program<any>;
  constructor({
    connection, // lightWasm,
    // _merkleTreeAccount,
  } /// TODO: implement once we enable ZKP verification
  : {
    // _merkleTreeAccount?: PublicKey;

    connection: Connection;
    // lightWasm: LightWasm;
  }) {
    this.connection = connection;

    // const solMerkleTree = new SolMerkleTree({
    //   lightWasm,
    //   pubkey: merkleTreeAccount,
    // });
    // this.merkleTrees.push(solMerkleTree);
    // this.lightWasm = lightWasm;
    // this.accountCompressionProgram = new Program(
    //   // @ts-ignore: idl is broken
    //   IDL,
    //   merkleTreeProgramId,
    //   new AnchorProvider(connection, {} as any, {}) // doesnt support browser
    // );
  }

  /// TODO: make implement rpc-interface
  // async getUtxosByOwner(
  //   utxoHash: bigint254,
  //   _config?: GetUtxoConfig
  // ): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[] | null> {
  //   // get indexed transactions
  //   const parsedtxs = await fetchRecentPublicTransactions({
  //     connection: this.connection,
  //     batchOptions: {
  //       limit: 1000,
  //     },
  //   });

  //   if (res.result.value === null) {
  //     return null;
  //   }

  //   const context: MerkleUpdateContext = {
  //     slotUpdated: res.result.value.slotUpdated,
  //     seq: res.result.value.seq,
  //   };

  //   const value: UtxoWithMerkleContext = {
  //     owner: res.result.value.owner,
  //     lamports: res.result.value.lamports,
  //     data: res.result.value.data,
  //     hash: utxoHash,
  //     merkleTree: res.result.value.merkleTree,
  //     leafIndex: res.result.value.leafIndex,
  //     address: res.result.value.address,
  //     stateNullifierQueue: res.result.value.stateNullifierQueue,
  //   };

  //   return { context, value };
  // }

  /**
   * Indexes light transactions by:
   * - getting all signatures the merkle tree was involved in
   * - trying to extract and parse event cpi for every signature's transaction
   * - if there are indexed transactions already in the rpc object only transactions after the last indexed event are indexed
   */
  async getIndexedEvents(): Promise<any[]> {
    // const merkleTreeAccountInfo = await connection.getAccountInfo(
    //   this.accounts.merkleTreeAccount,
    //   "confirmed",
    // );
    // if (!merkleTreeAccountInfo)
    //   throw new Error("Failed to fetch merkle tree account");
    // const coder = new BorshAccountsCoder(IDL_LIGHT_MERKLE_TREE_PROGRAM);
    // const merkleTreeAccount = coder.decode(
    //   "merkleTreeAccount",
    //   merkleTreeAccountInfo.data,
    // );
    // const stateMerkleTree = serializeOnchainMerkleTree(
    //   merkleTreeAccount.stateMerkleTree,
    // );

    // limits the number of signatures which are queried
    // if the number is too low it is not going to index all transactions
    // hence the dependency on the merkle tree account index times 260 transactions
    // which is approximately the number of transactions sent to send one compressed transaction and update the merkle tree
    const limit = 1000; //+ 260 * stateMerkleTree.nextIndex.toNumber();
    // if (this.indexedTransactions.length === 0) {
    const { transactions: newTransactions } =
      await fetchRecentPublicTransactions({
        connection: this.connection,
        batchOptions: {
          limit,
        },
      });
    this.indexedTransactions = newTransactions;
    return this.indexedTransactions;
    // }
    // else {
    //   if (this.indexedTransactions.length === 0) return [];

    //   const mostRecentTransaction = this.indexedTransactions.reduce((a, b) =>
    //     (a.transaction as ParsedIndexedTransaction).blockTime >
    //     (b.transaction as ParsedIndexedTransaction).blockTime
    //       ? a
    //       : b
    //   );

    //   const { transactions: newTransactions } = await fetchRecentTransactions({
    //     connection,
    //     batchOptions: {
    //       limit,
    //       until: (mostRecentTransaction.transaction as ParsedIndexedTransaction)
    //         .signature,
    //     },
    //   });
    //   this.indexedTransactions = [
    //     ...this.indexedTransactions,
    //     ...newTransactions,
    //   ];
    //   return this.indexedTransactions;
    // }
  }

  // async syncMerkleTree(
  //   merkleTreePubkey: PublicKey,
  //   indexedTransactions: ParsedIndexedTransaction[]
  // ): Promise<SolMerkleTree> {
  //   const solMerkleTreeIndex = this.merkleTrees.findIndex((tree) =>
  //     tree.pubkey.equals(merkleTreePubkey)
  //   );
  //   const rebuiltMt = await SolMerkleTree.build({
  //     lightWasm: this.lightWasm,
  //     pubkey: merkleTreePubkey,
  //     indexedTransactions,
  //   });
  //   this.merkleTrees[solMerkleTreeIndex] = rebuiltMt;
  //   return rebuiltMt;
  // }

  // async getEventById(
  //   merkleTreePdaPublicKey: PublicKey,
  //   id: string,
  //   _variableNameID: number
  // ): Promise<RpcIndexedTransactionResponse | undefined> {
  //   const indexedTransactions = await this.getIndexedEvents(
  //     this.connection
  //   );
  //   const indexedTransaction = indexedTransactions.find((trx) =>
  //     trx.IDs.includes(id)
  //   )?.transaction;
  //   if (!indexedTransaction) return undefined;
  //   const merkleTree = await this.syncMerkleTree(
  //     merkleTreePdaPublicKey,
  //     indexedTransactions.map(
  //       (trx) => trx.transaction as ParsedIndexedTransaction
  //     )
  //   );
  //   return createRpcIndexedTransactionResponse(
  //     indexedTransaction as ParsedIndexedTransaction,
  //     merkleTree
  //   );
  // }

  // async getEventsByIdBatch(
  //   ids: string[],
  //   variableNameID: number
  // ): Promise<RpcIndexedTransactionResponse[] | undefined> {
  //   const indexedTransactions = await this.getIndexedEvents(
  //     this.connection
  //   );
  //   const indexedTransactionsById = indexedTransactions.filter((trx) =>
  //     trx.IDs.some((id) => ids.includes(id))
  //   );
  //   const merkleTree = await this.syncMerkleTree(
  //     this.accounts.merkleTreeAccount,
  //     indexedTransactions.map(
  //       (trx) => trx.transaction as ParsedIndexedTransaction
  //     )
  //   );
  //   return indexedTransactionsById.map((trx) =>
  //     createRpcIndexedTransactionResponse(
  //       trx.transaction as ParsedIndexedTransaction,
  //       merkleTree
  //     )
  //   );
  // }

  // async getMerkleProofByIndexBatch(
  //   indexes: number[]
  // ): Promise<
  //   { merkleProofs: string[][]; root: string; index: number } | undefined
  // > {
  //   const indexedTransactions = await this.getIndexedEvents(
  //     this.connection
  //   );
  //   const merkleTree = await this.syncMerkleTree(
  //     this.accounts.merkleTreeAccount,
  //     indexedTransactions.map(
  //       (trx) => trx.transaction as ParsedIndexedTransaction
  //     )
  //   );
  //   if (!merkleTree) return undefined;
  //   const rootIndex = await getRootIndex(
  //     this.accountCompressionProgram,
  //     merkleTree.pubkey,
  //     merkleTree.merkleTree.root(),
  //     "merkleTreeAccount"
  //   );

  //   if (rootIndex === undefined) {
  //     throw new RpcError(
  //       TransactionErrorCode.ROOT_NOT_FOUND,
  //       "getRootIndex",
  //       `Root index not found for root ${merkleTree.merkleTree.root()}`
  //     );
  //   }
  //   return {
  //     merkleProofs: indexes.map(
  //       (index) => merkleTree.merkleTree.path(index).pathElements
  //     ),
  //     root: merkleTree.merkleTree.root(),
  //     index: rootIndex.toNumber(),
  //   };
  // }

  // async getMerkleRoot(
  //   merkleTreePubkey: PublicKey
  // ): Promise<{ root: string; index: number } | undefined> {
  //   const indexedTransactions = await this.getIndexedEvents(
  //     this.connection
  //   );
  //   const merkleTree = await this.syncMerkleTree(
  //     this.accounts.merkleTreeAccount,
  //     indexedTransactions.map(
  //       (trx) => trx.transaction as ParsedIndexedTransaction
  //     )
  //   );

  //   const rootIndex = await getRootIndex(
  //     this.accountCompressionProgram,
  //     merkleTreePubkey,
  //     merkleTree.merkleTree.root(),
  //     "merkleTreeAccount"
  //   );
  //   if (rootIndex === undefined) {
  //     throw new RpcError(
  //       TransactionErrorCode.ROOT_NOT_FOUND,
  //       "getRootIndex",
  //       `Root index not found for root ${merkleTree.merkleTree.root()}`
  //     );
  //   }
  //   return { root: merkleTree.merkleTree.root(), index: rootIndex.toNumber() };
  // }
}

// export const createRpcIndexedTransactionResponse = (
//   indexedTransaction: ParsedIndexedTransaction,
//   merkleTree: SolMerkleTree
// ): RpcIndexedTransactionResponse => {
//   const leavesIndexes = indexedTransaction.leaves.map((leaf) =>
//     merkleTree.merkleTree.indexOf(new BN(leaf).toString())
//   );
//   const merkleProofs = leavesIndexes.map(
//     (index) => merkleTree.merkleTree.path(index).pathElements
//   );
//   const rpcIndexedTransactionResponse: RpcIndexedTransactionResponse = {
//     transaction: indexedTransaction,
//     leavesIndexes,
//     merkleProofs,
//   };
//   return rpcIndexedTransactionResponse;
// };
