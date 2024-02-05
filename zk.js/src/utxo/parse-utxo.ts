import {
  FixableBeetStruct,
  array,
  bignum,
  coption,
  fixedSizeUint8Array,
  u64,
  u8,
  uniformFixedSizeArray,
} from "@metaplex-foundation/beet";
import { publicKey } from "@metaplex-foundation/beet-solana";
import { PublicKey } from "@solana/web3.js";
import { UTXO_PREFIX_LENGTH } from "../constants";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  OutUtxo,
  Utxo,
  convertParsingUtxoBeetToOutUtxo,
  outUtxoToUtxo,
} from ".";
import { PublicTransactionIndexerEventBeet } from "../transaction";
import { LightWasm } from "@lightprotocol/account.rs";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

export class ParsingUtxoBeet {
  constructor(
    readonly version: bignum,
    readonly poolType: bignum,
    readonly amounts: bignum[],
    readonly splAssetMint: PublicKey,
    readonly owner: Uint8Array,
    readonly blinding: Uint8Array,
    readonly dataHash: Uint8Array,
    readonly metaHash: Uint8Array,
    readonly address: Uint8Array,
    readonly message: number[] | null,
  ) {}

  static readonly struct = new FixableBeetStruct<
    ParsingUtxoBeet,
    ParsingUtxoBeet
  >(
    [
      ["version", u64],
      ["poolType", u64],
      ["amounts", uniformFixedSizeArray(u64, 2)],
      ["splAssetMint", coption(publicKey)],
      ["owner", fixedSizeUint8Array(32)],
      ["blinding", fixedSizeUint8Array(32)],
      ["dataHash", fixedSizeUint8Array(32)],
      ["metaHash", fixedSizeUint8Array(32)],
      ["address", fixedSizeUint8Array(32)],
      ["message", coption(array(u8))],
    ],
    (args) =>
      new ParsingUtxoBeet(
        args.version,
        args.poolType,
        args.amounts,
        args.splAssetMint,
        args.owner,
        args.blinding,
        args.dataHash,
        args.metaHash,
        args.address,
        args.message,
      ),
    "ParsingUtxo",
  );
}

export const getIdsFromEncryptedUtxos = (
  encryptedUtxos: Buffer,
  _numberOfLeaves: number,
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

export const eventsToOutUtxos = (
  events: PublicTransactionIndexerEventBeet[],
  lightWasm: LightWasm,
) => {
  const utxos: {
    outUtxo: OutUtxo;
    index: number;
    merkleTreePubkey: PublicKey | undefined;
  }[] = [];
  events.forEach((event) => {
    event.outUtxos.forEach((beetOutUtxo: ParsingUtxoBeet, i) => {
      if (
        utxos.find(
          ({ index }) => index === Number(event.outUtxoIndexes[i].toString()),
        ) === undefined
      ) {
        const outUtxo: OutUtxo | undefined = convertParsingUtxoBeetToOutUtxo(
          beetOutUtxo,
          lightWasm,
        );
        if (outUtxo) {
          utxos.push({
            outUtxo,
            index: Number(event.outUtxoIndexes[i].toString()),
            // TODO: add Merkle tree pubkey to support dynamic merkle trees
            merkleTreePubkey: undefined,
          });
        }
      }
    });
  });
  return utxos;
};

export const outUtxosToUtxos = (
  outUtxos: {
    outUtxo: OutUtxo;
    index: number;
    merkleTreePubkey: PublicKey | undefined;
  }[],
  lightWasm: LightWasm,
  merkleTree: MerkleTree,
) => {
  const utxos: Utxo[] = [];
  outUtxos.forEach(({ outUtxo, index }) => {
    const utxo = outUtxoToUtxo({
      outUtxo,
      merkleProof: merkleTree.path(index).pathElements,
      merkleTreeLeafIndex: index,
      lightWasm,
    });
    utxos.push(utxo);
  });

  return utxos;
};
