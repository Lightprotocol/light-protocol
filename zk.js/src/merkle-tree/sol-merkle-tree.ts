import { BN, Program, Provider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
import { LightWasm } from "@lightprotocol/account.rs";
import { ParsedIndexedTransaction } from "../types";
import {
  merkleTreeProgramId,
  MERKLE_TREE_HEIGHT,
  MERKLE_TREE_ROOTS,
  BN_0,
} from "../constants";
import { SolMerkleTreeError, SolMerkleTreeErrorCode } from "../errors";
import { IDL_PSP_ACCOUNT_COMPRESSION, LightMerkleTreeProgram } from "../idls";
import { sleep } from "../utils";
import { Utxo } from "../utxo";

const ffjavascript = require("ffjavascript");
const { unstringifyBigInts, beInt2Buff, leInt2Buff } = ffjavascript.utils;

const INDEX_SIZE = 8;
const ROOT_SIZE = 32;

export class OnchainMerkleTree {
  public nextIndex: BN;
  public roots: Uint8Array[];

  constructor({ nextIndex, roots }: { nextIndex: BN; roots: Uint8Array[] }) {
    this.nextIndex = nextIndex;
    this.roots = roots;
  }
}

export function serializeRoots(numbers: number[]): Uint8Array[] {
  const byteArray: Uint8Array[] = [];

  for (let i = 0; i < numbers.length; i += ROOT_SIZE) {
    const chunk = numbers.slice(i, i + ROOT_SIZE);
    byteArray.push(Uint8Array.from(chunk));
  }

  return byteArray;
}

export function serializeOnchainMerkleTree(bytes: number[]): OnchainMerkleTree {
  const nextIndex = new BN(Buffer.from(bytes.slice(0, INDEX_SIZE)), 10, "le");
  const roots = serializeRoots(
    bytes.slice(INDEX_SIZE, MERKLE_TREE_ROOTS * ROOT_SIZE),
  );

  return new OnchainMerkleTree({
    nextIndex,
    roots,
  });
}

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
      IDL_PSP_ACCOUNT_COMPRESSION,
      merkleTreeProgramId,
      provider,
    );

    let onchainMerkleTreeSet =
      await merkleTreeProgram.account.merkleTreeSet.fetch(pubkey, "processed");
    let onchainStateMerkleTree = serializeOnchainMerkleTree(
      onchainMerkleTreeSet.stateMerkleTree,
    );

    indexedTransactions.sort(
      (a, b) =>
        new BN(a.firstLeafIndex, "hex").toNumber() -
        new BN(b.firstLeafIndex, "hex").toNumber(),
    );
    const merkleTreeIndex = onchainStateMerkleTree.nextIndex;
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
    let index = onchainStateMerkleTree.roots.findIndex((root) => {
      return Array.from(builtMerkleTreeRoot).toString() === root.toString();
    });
    let retries = 3;
    while (index < 0 && retries > 0) {
      await sleep(100);
      retries--;
      onchainMerkleTreeSet =
        await merkleTreeProgram.account.merkleTreeSet.fetch(
          pubkey,
          "processed",
        );
      onchainStateMerkleTree = serializeOnchainMerkleTree(
        onchainMerkleTreeSet.stateMerkleTree,
      );
      index = onchainStateMerkleTree.roots.findIndex((root) => {
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
