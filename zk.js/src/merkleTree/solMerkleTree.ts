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
  BN_0, Poseidon,
} from "../index";
import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  LightMerkleTreeProgram,
} from "../idls/index";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
const anchor = require("@coral-xyz/anchor");
const ffjavascript = require("ffjavascript");
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
    const merkleTreeIndex = mtFetched.nextIndex;
    // ProgramAccount<MerkleTreeProgram["accounts"][7]>
    const leavesAccounts: Array<any> =
      await merkleTreeProgram.account.twoLeavesBytesPda.all();
    return { leavesAccounts, merkleTreeIndex, mtFetched };
  }

  static async build({
    pubkey,
    poseidon,
    indexedTransactions,
    provider,
  }: {
    pubkey: PublicKey;
    poseidon: Poseidon;
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
        new BN(a.firstLeafIndex).toNumber() -
        new BN(b.firstLeafIndex).toNumber(),
    );
    const merkleTreeIndex = mtFetched.nextIndex;
    const leaves: string[] = [];
    if (indexedTransactions.length > 0) {
      for (let i: number = 0; i < indexedTransactions.length; i++) {
        if (
          new BN(indexedTransactions[i].firstLeafIndex).toNumber() <
          merkleTreeIndex.toNumber()
        ) {
          for (const iterator of indexedTransactions[i].leaves) {
            leaves.push(new anchor.BN(iterator, undefined, "le").toString());
          }
        }
      }
    }

    const fetchedMerkleTree = new MerkleTree(
      MERKLE_TREE_HEIGHT,
      poseidon,
      leaves,
    );

    // @ts-ignore: unknown type error
    let index = mtFetched.roots.findIndex((root) => {
      return (
        Array.from(
          leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
          // @ts-ignore: unknown type error
        ).toString() === root.toString()
      );
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
      index = mtFetched.roots.findIndex((root) => {
        return (
          Array.from(
            leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
            // @ts-ignore: unknown type error
          ).toString() === root.toString()
        );
      });
    }

    if (index < 0) {
      throw new Error(
        `building merkle tree from chain failed: root local ${Array.from(
          leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32),
        ).toString()} is not present in roots fetched`,
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

  /**
   * @description Gets the merkle proofs for every input utxo with amounts > 0.
   * @description For input utxos with amounts == 0 it returns merkle paths with all elements = 0.
   */
  getMerkleProofs(
    poseidon: Poseidon,
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
        inputUtxo.index = this.merkleTree.indexOf(
          inputUtxo.getCommitment(poseidon),
        );

        if (inputUtxo.index || inputUtxo.index === 0) {
          if (inputUtxo.index < 0) {
            throw new SolMerkleTreeError(
              SolMerkleTreeErrorCode.INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE,
              "getMerkleProofs",
              `Input commitment ${inputUtxo.getCommitment(
                poseidon,
              )} was not found. Was the local merkle tree synced since the utxo was inserted?`,
            );
          }
          inputMerklePathIndices.push(inputUtxo.index.toString());
          inputMerklePathElements.push(
            this.merkleTree.path(inputUtxo.index).pathElements,
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
