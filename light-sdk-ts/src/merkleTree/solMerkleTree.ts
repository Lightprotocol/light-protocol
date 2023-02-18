import { BN, Program } from "@coral-xyz/anchor";
import { GetVersionedTransactionConfig, PublicKey } from "@solana/web3.js";
import {
  merkleTreeProgramId,
  IDL_MERKLE_TREE_PROGRAM,
  MERKLE_TREE_HEIGHT,
} from "../index";
import { MerkleTreeProgram } from "../idls/merkle_tree_program";
import { MerkleTree } from "./merkleTree";
import { program } from "@coral-xyz/anchor/dist/cjs/native/system";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import { lstat } from "fs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
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
    const merkleTreeProgram: Program<MerkleTreeProgram> = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
    );
    const mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      merkleTreePubkey,
    );
    const merkleTreeIndex = mtFetched.nextIndex;
    var leavesAccounts: Array<{
      pubkey: PublicKey;
      account: Account<Buffer>;
    }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
    return { leavesAccounts, merkleTreeIndex, mtFetched };
  }

  static async getCompressedLeaves(merkleTreePubkey: PublicKey) {
    const merkleTreeProgram: Program<MerkleTreeProgram> = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
    );
    const mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      merkleTreePubkey,
    );
    const merkleTreeIndex = mtFetched.nextIndex;
    // console.log(
    //   "getCompressedLeaves merkleTreeIndex",
    //   merkleTreeIndex.toNumber(),
    // );
    let leavesAccounts: Array<{
      account: Account<Buffer>;
    }> = new Array();
    let signatures = Array.from(
      await merkleTreeProgram.provider.connection.getSignaturesForAddress(
        merkleTreeProgram.programId,
        {},
        "confirmed",
      ),
      (x) => x.signature,
    );

    let config: GetVersionedTransactionConfig = {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    };
    // cannot request an unlimited amount of transactions from signatures
    for (var i = 0; i < signatures.length; i += 60) {
      var sigLen = 60;
      if (signatures.length < i + sigLen) {
        sigLen = signatures.length - i;
      }
      let transactions =
        await merkleTreeProgram.provider.connection.getTransactions(
          signatures.slice(i, i + sigLen),
          config,
        );

      let filteredTx = transactions.filter((tx) => {
        try {
          const tmp = tx?.transaction.message.accountKeys.map((key) =>
            key.toBase58(),
          );
          return tmp?.includes(SPL_NOOP_ADDRESS);
        } catch {}
      });
      filteredTx.map((tx) => {
        tx?.meta?.innerInstructions?.map((ix) => {
          ix.instructions.map((ixInner) => {
            const data = bs58.decode(ixInner.data);
            leavesAccounts.push({
              account: {
                leftLeafIndex: new BN(
                  data.slice(96 + 256, 96 + 256 + 8),
                  undefined,
                  "le",
                ),
                encryptedUtxos: Array.from(data.slice(96, 96 + 256)),
                nodeLeft: Array.from(data.slice(0, 32)),
                nodeRight: Array.from(data.slice(32, 64)),
                merkleTreePubkey: new PublicKey(Array.from(data.slice(64, 96))),
              },
            });
          });
        });
      });
    }

    return { leavesAccounts, merkleTreeIndex, mtFetched };
  }

  static async build({
    pubkey,
    poseidon,
  }: {
    pubkey: PublicKey; // pubkey to bytes
    poseidon: any;
  }) {
    const { leavesAccounts, merkleTreeIndex, mtFetched } =
      await SolMerkleTree.getCompressedLeaves(pubkey);

    leavesAccounts.sort(
      (a, b) =>
        a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber(),
    );

    const leaves: string[] = [];
    if (leavesAccounts.length > 0) {
      for (let i: number = 0; i < leavesAccounts.length; i++) {
        if (
          leavesAccounts[i].account.leftLeafIndex.toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          leaves.push(
            new anchor.BN(
              leavesAccounts[i].account.nodeLeft,
              undefined,
              "le",
            ).toString(),
          ); // .reverse()
          leaves.push(
            new anchor.BN(
              leavesAccounts[i].account.nodeRight,
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
      publicKey: PublicKey;
      account: any;
    }>
  > {
    const { leavesAccounts, merkleTreeIndex } = await SolMerkleTree.getLeaves(
      merkleTreePubkey,
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

  static async getUninsertedLeavesRelayer(merkleTreePubkey: PublicKey) {
    return (await SolMerkleTree.getUninsertedLeaves(merkleTreePubkey)).map(
      (pda) => {
        return { isSigner: false, isWritable: true, pubkey: pda.publicKey };
      },
    );
  }

  static async getInsertedLeaves(
    merkleTreePubkey: PublicKey,
  ) /*: Promise<{ pubkey: PublicKey; account: Account<Buffer>; }[]>*/ {
    const { leavesAccounts, merkleTreeIndex } =
      await SolMerkleTree.getCompressedLeaves(merkleTreePubkey);

    let filteredLeaves = leavesAccounts
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
