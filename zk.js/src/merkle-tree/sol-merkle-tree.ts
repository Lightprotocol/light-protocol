import { BN, Program, Provider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
import { LightWasm } from "@lightprotocol/account.rs";
import { ParsedIndexedTransaction } from "../types";
import { merkleTreeProgramId, MERKLE_TREE_HEIGHT, BN_0 } from "../constants";
import { SolMerkleTreeError, SolMerkleTreeErrorCode } from "../errors";
import { IDL_LIGHT_MERKLE_TREE_PROGRAM, LightMerkleTreeProgram } from "../idls";
import { sleep } from "../utils";
import { Utxo } from "../utxo";

const ffjavascript = require("ffjavascript");
const { unstringifyBigInts, beInt2Buff, leInt2Buff } = ffjavascript.utils;

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
  lightWasm: LightWasm;

  constructor({
    pubkey,
    lightWasm,
    merkleTree = new MerkleTree(MERKLE_TREE_HEIGHT, lightWasm),
  }: {
    lightWasm: LightWasm;
    merkleTree?: MerkleTree;
    pubkey: PublicKey;
  }) {
    this.pubkey = pubkey;
    this.lightWasm = lightWasm;
    this.merkleTree = merkleTree;
  }

  static async getLeaves(merkleTreePubkey: PublicKey, provider?: Provider) {
    const merkleTreeProgram: Program<LightMerkleTreeProgram> = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider,
    );
    const mtFetched = await merkleTreeProgram.account.merkleTreeSet.fetch(
      merkleTreePubkey,
      "processed",
    );
    const merkleTreeIndex = mtFetched.stateMerkleTree.nextIndex;
    // ProgramAccount<MerkleTreeProgram["accounts"][7]>
    return { merkleTreeIndex, mtFetched };
  }

  static async build({
    lightWasm,
    pubkey,
    indexedTransactions,
    provider,
  }: {
    lightWasm: LightWasm;
    pubkey: PublicKey;
    indexedTransactions: ParsedIndexedTransaction[];
    provider?: Provider;
  }) {
    const merkleTreeProgram: Program<LightMerkleTreeProgram> = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider,
    );

    let mtFetched = await merkleTreeProgram.account.merkleTreeSet.fetch(
      pubkey,
      "processed",
    );

    indexedTransactions.sort(
      (a, b) =>
        new BN(a.firstLeafIndex, "hex").toNumber() -
        new BN(b.firstLeafIndex, "hex").toNumber(),
    );
    const merkleTreeIndex = mtFetched.stateMerkleTree.nextIndex;
    const leaves: string[] = [];
    if (indexedTransactions.length > 0) {
      for (let i: number = 0; i < indexedTransactions.length; i++) {
        if (
          new BN(indexedTransactions[i].firstLeafIndex, "hex").toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          for (const iterator of indexedTransactions[i].leaves) {
            leaves.push(new BN(iterator, undefined, "be").toString());
          }
        }
      }
    }

    const builtMerkleTree = new MerkleTree(
      MERKLE_TREE_HEIGHT,
      lightWasm,
      leaves,
    );

    const builtMerkleTreeRoot = beInt2Buff(
      unstringifyBigInts(builtMerkleTree.root()),
      32,
    );
    let index = mtFetched.stateMerkleTree.roots.findIndex((root) => {
      return Array.from(builtMerkleTreeRoot).toString() === root.toString();
    });
    let retries = 3;
    while (index < 0 && retries > 0) {
      await sleep(100);
      retries--;
      mtFetched = await merkleTreeProgram.account.merkleTreeSet.fetch(
        pubkey,
        "processed",
      );
      index = mtFetched.stateMerkleTree.roots.findIndex((root) => {
        return Array.from(builtMerkleTreeRoot).toString() === root.toString();
      });
    }

    if (index < 0) {
      throw new Error(
        `building merkle tree from chain failed: root local ${Array.from(
          builtMerkleTreeRoot,
        ).toString()} is not present in roots fetched`,
      );
    }

    return new SolMerkleTree({
      pubkey,
      lightWasm,
      merkleTree: builtMerkleTree,
    });
  }
}
