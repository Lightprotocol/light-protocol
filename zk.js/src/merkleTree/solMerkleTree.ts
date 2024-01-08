import { BN, Program, Provider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import {
  merkleTreeProgramId,
  MERKLE_TREE_HEIGHT,
  sleep,
  ParsedIndexedTransaction,
  SolMerkleTreeErrorCode,
  SolMerkleTreeError,
  Utxo,
  BN_0,
} from "../index";
import { LightWasm } from "@lightprotocol/account.rs";
import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  LightMerkleTreeProgram,
} from "../idls/index";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
const anchor = require("@coral-xyz/anchor");
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
    const mtFetched =
      await merkleTreeProgram.account.transactionMerkleTree.fetch(
        merkleTreePubkey,
        "processed",
      );
    const merkleTreeIndex = mtFetched.merkleTree.nextIndex;
    // ProgramAccount<MerkleTreeProgram["accounts"][7]>
    const leavesAccounts: Array<any> =
      await merkleTreeProgram.account.twoLeavesBytesPda.all();
    return { leavesAccounts, merkleTreeIndex, mtFetched };
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

    let mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(
      pubkey,
      "processed",
    );

    indexedTransactions.sort(
      (a, b) =>
        new BN(a.firstLeafIndex, "hex").toNumber() -
        new BN(b.firstLeafIndex, "hex").toNumber(),
    );
    const merkleTreeIndex = mtFetched.merkleTree.nextIndex;
    const leaves: string[] = [];
    if (indexedTransactions.length > 0) {
      for (let i: number = 0; i < indexedTransactions.length; i++) {
        if (
          new BN(indexedTransactions[i].firstLeafIndex, "hex").toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          for (const iterator of indexedTransactions[i].leaves) {
            leaves.push(new anchor.BN(iterator, undefined, "be").toString());
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
    let index = mtFetched.merkleTree.roots.findIndex((root) => {
      return Array.from(builtMerkleTreeRoot).toString() === root.toString();
    });
    let retries = 3;
    while (index < 0 && retries > 0) {
      await sleep(100);
      retries--;
      mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(
        pubkey,
        "processed",
      );
      // @ts-ignore: unknown type error
      index = mtFetched.merkleTree.roots.findIndex((root) => {
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

    const filteredLeaves = leavesAccounts
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

  static async getUninsertedLeavesRpc(
    merkleTreePubkey: PublicKey,
    provider?: Provider,
  ) {
    return (
      await SolMerkleTree.getUninsertedLeaves(merkleTreePubkey, provider)
    ).map((pda) => {
      return { isSigner: false, isWritable: true, pubkey: pda.publicKey };
    });
  }

  /**
   * @description Gets the merkle proofs for every input utxo with amounts > 0.
   * @description For input utxos with amounts == 0 it returns merkle paths with all elements = 0.
   */
  getMerkleProofs(
    lightWasm: LightWasm,
    inputUtxos: Utxo[],
  ): {
    inputMerklePathIndices: Array<string>;
    inputMerklePathElements: Array<Array<string>>;
  } {
    const inputMerklePathIndices = new Array<string>();
    const inputMerklePathElements = new Array<Array<string>>();
    // getting merkle proofs
    for (const inputUtxo of inputUtxos) {
      if (inputUtxo.amounts[0].gt(BN_0) || inputUtxo.amounts[1].gt(BN_0)) {
        inputUtxo.merkleTreeLeafIndex = this.merkleTree.indexOf(
          inputUtxo.utxoHash,
        );

        if (
          inputUtxo.merkleTreeLeafIndex ||
          inputUtxo.merkleTreeLeafIndex === 0
        ) {
          if (inputUtxo.merkleTreeLeafIndex < 0) {
            throw new SolMerkleTreeError(
              SolMerkleTreeErrorCode.INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE,
              "getMerkleProofs",
              `Input commitment ${inputUtxo.utxoHash} was not found. Was the local merkle tree synced since the utxo was inserted?`,
            );
          }
          inputMerklePathIndices.push(inputUtxo.merkleTreeLeafIndex.toString());
          inputMerklePathElements.push(
            this.merkleTree.path(inputUtxo.merkleTreeLeafIndex).pathElements,
          );
        }
      } else {
        inputMerklePathIndices.push("0");
        inputMerklePathElements.push(
          new Array<string>(this.merkleTree.levels).fill("0"),
        );
      }
    }
    return { inputMerklePathIndices, inputMerklePathElements };
  }
}
