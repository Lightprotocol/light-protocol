import { PublicKey } from "@solana/web3.js";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import {
  Utxo,
  UtxoWithMerkleContext,
  addMerkleContextToUtxo,
  createUtxo,
} from "./utxo";
import { TlvSerial, deserializeTlv, serializeTlv } from "./utxo-data";
import {
  arrayToBigint,
  bufToDecStr,
  hashToBn254FieldSizeLe,
} from "../utils/conversion";
import { bigint254 } from "./bigint254";

export type InputUtxoSerial = {
  owner: number;
  leafIndex: number;
  lamports: number;
  data: TlvSerial | null;
};

export type OutputUtxoSerial = {
  owner: number;
  lamports: number;
  data: TlvSerial | null;
};

export interface SerializedUtxos {
  pubkeyArray: PublicKey[];
  u64Array: bigint[];
  inUtxos: [InputUtxoSerial, number, number][];
  outUtxos: [OutputUtxoSerial, number][];
}

export class UtxoSerde {
  static async addInUtxos(
    serializedUtxos: SerializedUtxos,
    utxosToAdd: Utxo[],
    accounts: PublicKey[],
    leafIndices: number[],
    inUtxoMerkleTreePubkeys: PublicKey[],
    nullifierArrayPubkeys: PublicKey[]
  ): Promise<SerializedUtxos> {
    if (serializedUtxos.inUtxos.length > 0) {
      throw new Error("InUtxosAlreadyAdded");
    }
    if (
      utxosToAdd.length !== leafIndices.length ||
      utxosToAdd.length !== inUtxoMerkleTreePubkeys.length ||
      utxosToAdd.length !== nullifierArrayPubkeys.length
    ) {
      throw new Error("ArrayLengthMismatch");
    }

    const utxos: [InputUtxoSerial, number, number][] = [];
    const merkleTreeIndices = new Map<string, number>();
    const nullifierIndices = new Map<string, number>();

    utxosToAdd.forEach((utxo, i) => {
      const ownerIndex = accounts.findIndex((acc) => acc.equals(utxo.owner));
      const owner =
        ownerIndex >= 0
          ? ownerIndex
          : serializedUtxos.pubkeyArray.push(utxo.owner) - 1 + accounts.length;
      const lamportsIndex = serializedUtxos.u64Array.findIndex(
        (l) => l === utxo.lamports
      );
      const lamports =
        lamportsIndex >= 0
          ? lamportsIndex
          : serializedUtxos.u64Array.push(BigInt(utxo.lamports)) - 1;

      const data = utxo.data
        ? serializeTlv(utxo.data, serializedUtxos.pubkeyArray, accounts)
        : null;

      const inUtxoSerializable: InputUtxoSerial = {
        owner,
        leafIndex: leafIndices[i],
        lamports,
        data,
      };

      // Calculate indices for merkle tree and nullifier array pubkeys
      let merkleTreeIndex = merkleTreeIndices.get(
        inUtxoMerkleTreePubkeys[i].toString()
      );
      if (merkleTreeIndex === undefined) {
        merkleTreeIndex = merkleTreeIndices.size;
        merkleTreeIndices.set(
          inUtxoMerkleTreePubkeys[i].toString(),
          merkleTreeIndex
        );
      }

      let nullifierIndex = nullifierIndices.get(
        nullifierArrayPubkeys[i].toString()
      );
      if (nullifierIndex === undefined) {
        nullifierIndex = nullifierIndices.size;
        nullifierIndices.set(
          nullifierArrayPubkeys[i].toString(),
          nullifierIndex
        );
      }

      utxos.push([inUtxoSerializable, merkleTreeIndex, nullifierIndex]);
    });

    // Extend SerializedUtxos
    serializedUtxos.inUtxos.push(...utxos);
    return serializedUtxos;
  }

  static async addOutUtxos(
    serializedUtxos: SerializedUtxos,
    utxosToAdd: Utxo[],
    accounts: PublicKey[],
    remainingAccountsPubkeys: PublicKey[],
    outUtxoMerkleTreePubkeys: PublicKey[]
  ): Promise<SerializedUtxos> {
    if (utxosToAdd.length === 0) return serializedUtxos;

    const utxos: [OutputUtxoSerial, number][] = [];
    const merkleTreeIndices = new Map<string, number>();
    const remainingAccountsIndices = new Map<string, number>();

    // Initialize indices for remaining accounts pubkeys
    remainingAccountsPubkeys.forEach((pubkey, index) => {
      remainingAccountsIndices.set(pubkey.toString(), index);
    });

    utxosToAdd.forEach((utxo, i) => {
      const ownerIndex = accounts.findIndex((acc) => acc.equals(utxo.owner));
      const owner =
        ownerIndex >= 0
          ? ownerIndex
          : serializedUtxos.pubkeyArray.findIndex((pubkey) =>
              pubkey.equals(utxo.owner)
            ) >= 0
          ? serializedUtxos.pubkeyArray.findIndex((pubkey) =>
              pubkey.equals(utxo.owner)
            ) + accounts.length
          : serializedUtxos.pubkeyArray.push(utxo.owner) - 1 + accounts.length;
      const lamportsIndex = serializedUtxos.u64Array.findIndex(
        (l) => l === BigInt(utxo.lamports)
      );
      const lamports =
        lamportsIndex >= 0
          ? lamportsIndex
          : serializedUtxos.u64Array.push(BigInt(utxo.lamports)) - 1;

      const data = utxo.data
        ? serializeTlv(utxo.data, serializedUtxos.pubkeyArray, accounts)
        : null;

      const outUtxoSerializable: OutputUtxoSerial = {
        owner,
        lamports,
        data,
      };

      // Calc index for merkle tree pubkey
      let merkleTreeIndex = merkleTreeIndices.get(
        outUtxoMerkleTreePubkeys[i].toString()
      );
      if (merkleTreeIndex === undefined) {
        merkleTreeIndex = remainingAccountsIndices.get(
          outUtxoMerkleTreePubkeys[i].toString()
        );
        if (merkleTreeIndex === undefined) {
          merkleTreeIndex =
            remainingAccountsIndices.size + merkleTreeIndices.size;
          merkleTreeIndices.set(
            outUtxoMerkleTreePubkeys[i].toString(),
            merkleTreeIndex
          );
        }
      }

      utxos.push([outUtxoSerializable, merkleTreeIndex]);
    });

    // Extend SerializedUtxos
    serializedUtxos.outUtxos.push(...utxos);
    return serializedUtxos;
  }

  static async deserializeInputUtxos(
    hasher: LightWasm,
    serializedUtxos: SerializedUtxos,
    accounts: PublicKey[],
    merkleTreeAccounts: PublicKey[]
  ): Promise<[UtxoWithMerkleContext, number, number][]> {
    const inputUtxos: [UtxoWithMerkleContext, number, number][] = [];

    serializedUtxos.inUtxos.forEach(async (inUtxoSerialized, i) => {
      const [inUtxo, indexMerkleTreeAccount, indexNullifierArrayAccount] =
        inUtxoSerialized;

      // resolve owner
      const owner =
        inUtxo.owner < accounts.length
          ? accounts[inUtxo.owner]
          : serializedUtxos.pubkeyArray[inUtxo.owner - accounts.length];

      // resolve lamports
      const lamports = serializedUtxos.u64Array[inUtxo.lamports];

      // resolve data
      const data = inUtxo.data
        ? deserializeTlv(inUtxo.data, [
            ...accounts,
            ...serializedUtxos.pubkeyArray,
          ])
        : undefined;

      // reconstruct inputUtxo
      const utxo = createUtxo(owner, lamports, data);
      const utxoHash = await createUtxoHash(
        hasher,
        utxo,
        merkleTreeAccounts[indexMerkleTreeAccount],
        inUtxo.leafIndex
      );
      const utxoWithMerkleContext = addMerkleContextToUtxo(
        utxo,
        utxoHash,
        merkleTreeAccounts[indexMerkleTreeAccount],
        inUtxo.leafIndex
      );

      inputUtxos.push([
        utxoWithMerkleContext,
        indexMerkleTreeAccount,
        indexNullifierArrayAccount,
      ]);
    });

    return inputUtxos;
  }

  static async deserializeOutputUtxos(
    serializedUtxos: SerializedUtxos,
    accounts: PublicKey[]
  ): Promise<[Utxo, number][]> {
    const outputUtxos: [Utxo, number][] = [];

    for (const [
      outUtxoSerialized,
      indexMerkleTreeAccount,
    ] of serializedUtxos.outUtxos) {
      // Resolve owner
      const owner =
        outUtxoSerialized.owner < accounts.length
          ? accounts[outUtxoSerialized.owner]
          : serializedUtxos.pubkeyArray[
              outUtxoSerialized.owner - accounts.length
            ];

      // Resolve lamports
      const lamports = serializedUtxos.u64Array[outUtxoSerialized.lamports];

      // Resolve data
      const data = outUtxoSerialized.data
        ? deserializeTlv(outUtxoSerialized.data, [
            ...accounts,
            ...serializedUtxos.pubkeyArray,
          ])
        : undefined;

      // Reconstruct Utxo
      const utxo = createUtxo(owner, lamports, data);

      outputUtxos.push([utxo, indexMerkleTreeAccount]);
    }

    return outputUtxos;
  }
}

/// TODO: bunch of redundant conversions. optimize.
/** Computes unique utxo value from merkleTree, leafIndex */
const computeBlinding = async (
  hasher: LightWasm,
  merkleTreePublicKey: PublicKey,
  leafIndex: number
): Promise<bigint254> => {
  /// ensure <254-bit
  const mtHash = await hashToBn254FieldSizeLe(merkleTreePublicKey.toBuffer());
  if (!mtHash) throw new Error("Failed to hash merkle tree public key");

  const mtPubkeyDecStr = bufToDecStr(mtHash[0]);
  const leafIndexDecStr = BigInt(leafIndex).toString();

  return hasher.poseidonHashBigint([mtPubkeyDecStr, leafIndexDecStr]);
};

/// TODO: bunch of redundant conversions. optimize.
/**
 * Hashes a UTXO preimage.
 * Hash inputs: owner, blinding(merkleTree,leafIndex), lamports, tlvDataHash
 *
 * async for browser crypto.digest support */
async function createUtxoHash(
  hasher: LightWasm,
  utxo: Utxo,
  merkleTree: PublicKey,
  leafIndex: number
): Promise<bigint254> {
  const { owner, lamports, data } = utxo;

  /// hash all tlv elements into a single hash
  const tlvDataHash = data
    ? hasher.poseidonHashString(data.map((d) => d.dataHash.toString()))
    : BigInt(0).toString();

  /// ensure <254-bit
  const ownerHash = await hashToBn254FieldSizeLe(owner.toBuffer());
  if (!ownerHash) throw new Error("Failed to hash owner public key");
  const ownerDecStr = bufToDecStr(ownerHash[0]);

  const lamportsDecStr = BigInt(lamports).toString();

  const blindingDecStr = (
    await computeBlinding(hasher, merkleTree, leafIndex)
  ).toString();

  return hasher.poseidonHashBigint([
    ownerDecStr,
    blindingDecStr,
    lamportsDecStr,
    tlvDataHash,
  ]);
}
