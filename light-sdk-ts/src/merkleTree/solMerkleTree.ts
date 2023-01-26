// @ts-nocheck
import { Program } from "@coral-xyz/anchor";
import { Account, PublicKey } from "@solana/web3.js";
import {
  merkleTreeProgramId,
  MerkleTreeProgramIdl,
  MERKLE_TREE_HEIGHT,
} from "../index";
import { MerkleTreeProgram } from "../idls/merkle_tree_program";
import { MerkleTree } from "./merkleTree";
const anchor = require("@coral-xyz/anchor");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

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

  static async getLeaves(merkleTreePubkey: PublicKey) {
    const merkleTreeProgram: Program<MerkleTreeProgramIdl> = new Program(
      MerkleTreeProgram,
      merkleTreeProgramId,
    );
    const mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      merkleTreePubkey,
    );
    const merkleTreeIndex = mtFetched.nextIndex;
    var leaveAccounts: Array<{
      pubkey: PublicKey;
      account: Account<Buffer>;
    }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
    return { leaveAccounts, merkleTreeIndex, mtFetched };
  }

  static async build({
    pubkey,
    poseidon,
  }: {
    pubkey: PublicKey; // pubkey to bytes
    poseidon: any;
  }) {
    const { leaveAccounts, merkleTreeIndex, mtFetched } =
      await SolMerkleTree.getLeaves(pubkey);

    leaveAccounts.sort(
      (a, b) =>
        a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber(),
    );

    const leaves: string[] = [];
    if (leaveAccounts.length > 0) {
      for (let i: number = 0; i < leaveAccounts.length; i++) {
        if (
          leaveAccounts[i].account.leftLeafIndex.toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          leaves.push(
            new anchor.BN(
              leaveAccounts[i].account.nodeLeft,
              undefined,
              "le",
            ).toString(),
          ); // .reverse()
          leaves.push(
            new anchor.BN(
              leaveAccounts[i].account.nodeRight,
              undefined,
              "le",
            ).toString(),
          );
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
      ).toString() != mtFetched.roots[mtFetched.currentRootIndex].toString()
    ) {
      throw new Error(
        `building merkle tree from chain failed: root local ${Array.from(
          leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
        ).toString()} != root fetched ${
          mtFetched.roots[mtFetched.currentRootIndex]
        }`,
      );
    }

    return new SolMerkleTree({ merkleTree: fetchedMerkleTree, pubkey });
  }

  static async getUninsertedLeaves(merkleTreePubkey: PublicKey): Promise<
    Array<{
      pubkey: PublicKey;
      account: Account<Buffer>;
    }>
  > {
    const { leaveAccounts, merkleTreeIndex } = await SolMerkleTree.getLeaves(
      merkleTreePubkey,
    );

    let filteredLeaves = leaveAccounts
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

  static async getUninsertedLeavesRelayer(merkleTreePubkey: PublicKey) {
    return (await SolMerkleTree.getUninsertedLeaves(merkleTreePubkey)).map(
      (pda) => {
        return { isSigner: false, isWritable: false, pubkey: pda.publicKey };
      },
    );
  }

  static async getInsertedLeaves(
    merkleTreePubkey: PublicKey,
  ) /*: Promise<{ pubkey: PublicKey; account: Account<Buffer>; }[]>*/ {
    const { leaveAccounts, merkleTreeIndex } = await SolMerkleTree.getLeaves(
      merkleTreePubkey,
    );

    console.log("Total nr of accounts. ", leaveAccounts.length);

    let filteredLeaves = leaveAccounts
      .filter((pda) => {
        return (
          pda.account.leftLeafIndex.toNumber() < merkleTreeIndex.toNumber()
        );
      })
      .sort(
        (a, b) =>
          a.account.leftLeafIndex.toNumber() -
          b.account.leftLeafIndex.toNumber(),
      );

    return filteredLeaves;
  }
}
