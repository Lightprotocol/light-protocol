import { BN, Program, Provider } from "@coral-xyz/anchor";
import { GetVersionedTransactionConfig, PublicKey } from "@solana/web3.js";
import {
  merkleTreeProgram,
  merkleTreeProgramId,
  MERKLE_TREE_HEIGHT,
  IndexedTransaction,
  indexRecentTransactions,
  Relayer,
} from "../index";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
import { MerkleTree } from "./merkleTree";
const anchor = require("@coral-xyz/anchor");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
export type QueuedLeavesPda = {
  leftLeafIndex: BN;
  encryptedUtxos: Array<number>;
  nodeLeft: Array<number>;
  nodeRight: Array<number>;
  merkleTreePubkey: PublicKey;
};

// TODO: once we have multiple trees add merkleTree[] and fetchTree(pubkey);
export class SolMerkleTree {
  merkleTree: MerkleTree;
  pubkey: PublicKey;

  constructor({
    pubkey,
    poseidon,
    merkleTree = new MerkleTree(MERKLE_TREE_HEIGHT, poseidon),
  }: {
    poseidon?: any;
    merkleTree?: MerkleTree;
    pubkey: PublicKey;
  }) {
    this.pubkey = pubkey;
    this.merkleTree = merkleTree;
  }

  static async getLeaves(merkleTreePubkey: PublicKey, provider?: Provider) {
    const merkleTreeProgram: Program<MerkleTreeProgram> = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider,
    );
    const mtFetched =
      await merkleTreeProgram.account.transactionMerkleTree.fetch(
        merkleTreePubkey,
        "confirmed",
      );
    const merkleTreeIndex = mtFetched.nextIndex;
    // ProgramAccount<MerkleTreeProgram["accounts"][7]>
    var leavesAccounts: Array<any> =
      await merkleTreeProgram.account.twoLeavesBytesPda.all();
    return { leavesAccounts, merkleTreeIndex, mtFetched };
  }

  static async build({
    pubkey,
    poseidon,
    indexedTransactions,
    provider,
  }: {
    pubkey: PublicKey; // pubkey to bytes
    poseidon: any;
    indexedTransactions: IndexedTransaction[];
    provider?: Provider;
  }) {
    const merkleTreeProgram: Program<MerkleTreeProgram> = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider,
    );

    const mtFetched =
      await merkleTreeProgram.account.transactionMerkleTree.fetch(
        pubkey,
        "confirmed",
      );

    const merkleTreeIndex = mtFetched.nextIndex;

    const leaves: string[] = [];
    if (indexedTransactions.length > 0) {
      for (let i: number = 0; i < indexedTransactions.length; i++) {
        if (
          indexedTransactions[i].firstLeafIndex.toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          for (const iterator of indexedTransactions[i].leaves) {
            leaves.push(new anchor.BN(iterator, undefined, "le").toString());
          }
        }
      }
    }

    let fetchedMerkleTree = new MerkleTree(
      MERKLE_TREE_HEIGHT,
      poseidon,
      leaves,
    );
    if (
      Array.from(
        leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
        // @ts-ignore: unknown type error
      ).toString() != mtFetched.roots[mtFetched.currentRootIndex].toString()
    ) {
      throw new Error(
        `building merkle tree from chain failed: root local ${Array.from(
          leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
        ).toString()} != root fetched ${
          // @ts-ignore: unknown type error
          mtFetched.roots[mtFetched.currentRootIndex]
        }`,
      );
    }

    return new SolMerkleTree({ merkleTree: fetchedMerkleTree, pubkey });
  }

  static async getUninsertedLeaves(
    merkleTreePubkey: PublicKey,
    provider?: Provider,
  ): Promise<
    Array<{
      publicKey: PublicKey;
      account: any;
    }>
  > {
    const { leavesAccounts, merkleTreeIndex } = await SolMerkleTree.getLeaves(
      merkleTreePubkey,
      provider,
    );

    let filteredLeaves = leavesAccounts
      .filter((pda) => {
        if (
          pda.account.merkleTreePubkey.toBase58() ===
          merkleTreePubkey.toBase58()
        ) {
          return (
            pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber()
          );
        }
      })
      .sort(
        (a, b) =>
          a.account.leftLeafIndex.toNumber() -
          b.account.leftLeafIndex.toNumber(),
      );
    return filteredLeaves;
  }

  static async getUninsertedLeavesRelayer(
    merkleTreePubkey: PublicKey,
    provider?: Provider,
  ) {
    return (
      await SolMerkleTree.getUninsertedLeaves(merkleTreePubkey, provider)
    ).map((pda) => {
      return { isSigner: false, isWritable: true, pubkey: pda.publicKey };
    });
  }
}
