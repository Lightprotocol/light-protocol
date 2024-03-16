import { PublicKey } from '@solana/web3.js';
import { LightWasm } from '@lightprotocol/account.rs';
import {
  Utxo,
  UtxoWithMerkleContext,
  addMerkleContextToUtxo,
  createUtxo,
} from './utxo';
import { deserializeTlv, serializeTlv } from './utxo-data';
import { bufToDecStr, hashToBn254FieldSizeLe } from '../utils/conversion';
import { BN254, bn, createBN254 } from './BN254';
import { BN } from '@coral-xyz/anchor';
import {
  InUtxoSerializableTuple_IdlType,
  InUtxoSerializable_IdlType,
  OutUtxoSerializableTuple,
  OutUtxoSerializable_IdlType,
  TlvDataElement_IdlType,
  Tlv_IdlType,
} from './types';

export class UtxoSerde {
  pubkeyArray: PublicKey[];
  u64Array: BN[]; // TODO: check encoding
  inUtxos: InUtxoSerializableTuple_IdlType[];
  outUtxos: OutUtxoSerializableTuple[];

  constructor() {
    this.pubkeyArray = [];
    this.u64Array = [];
    this.inUtxos = [];
    this.outUtxos = [];
  }
  addinputUtxos(
    utxosToAdd: Utxo[],
    accounts: PublicKey[],
    leafIndices: BN[],
    inputUtxoMerkleTreePubkeys: PublicKey[],
    nullifierArrayPubkeys: PublicKey[],
  ): UtxoSerde {
    if (this.inUtxos.length > 0) {
      throw new Error('inputUtxosAlreadyAdded');
    }
    if (
      utxosToAdd.length !== leafIndices.length ||
      utxosToAdd.length !== inputUtxoMerkleTreePubkeys.length ||
      utxosToAdd.length !== nullifierArrayPubkeys.length
    ) {
      throw new Error('ArrayLengthMismatch');
    }

    const utxos: InUtxoSerializableTuple_IdlType[] = [];
    const merkleTreeIndices = new Map<string, number>();
    const nullifierIndices = new Map<string, number>();

    utxosToAdd.forEach((utxo, i) => {
      const ownerIndex = accounts.findIndex((acc) => acc.equals(utxo.owner));
      const owner =
        ownerIndex >= 0
          ? ownerIndex
          : this.pubkeyArray.push(utxo.owner) - 1 + accounts.length;
      const lamportsIndex = this.u64Array.findIndex((l) => l === utxo.lamports);
      const lamports =
        lamportsIndex >= 0
          ? lamportsIndex
          : this.u64Array.push(bn(utxo.lamports)) - 1;

      const data = utxo.data
        ? serializeTlv(utxo.data, this.pubkeyArray, accounts)
        : null;

      const inputUtxoSerializable: InUtxoSerializable_IdlType = {
        owner,
        leafIndex: leafIndices[i].toNumber(),
        lamports,
        data,
        address: utxo.address,
      };

      // Calculate indices for merkle tree and nullifier array pubkeys
      let merkleTreeIndex = merkleTreeIndices.get(
        inputUtxoMerkleTreePubkeys[i].toString(),
      );
      if (merkleTreeIndex === undefined) {
        merkleTreeIndex = merkleTreeIndices.size;
        merkleTreeIndices.set(
          inputUtxoMerkleTreePubkeys[i].toString(),
          merkleTreeIndex,
        );
      }

      let nullifierIndex = nullifierIndices.get(
        nullifierArrayPubkeys[i].toString(),
      );
      if (nullifierIndex === undefined) {
        nullifierIndex = nullifierIndices.size;
        nullifierIndices.set(
          nullifierArrayPubkeys[i].toString(),
          nullifierIndex,
        );
      }

      utxos.push({
        inUtxoSerializable: inputUtxoSerializable,
        indexMtAccount: merkleTreeIndex,
        indexNullifierArrayAccount: nullifierIndex,
      });
    });

    // Extend SerializedUtxos
    this.inUtxos.push(...utxos);
    return this;
  }

  addoutputUtxos(
    utxosToAdd: Utxo[],
    accounts: PublicKey[],
    remainingAccountsPubkeys: PublicKey[],
    outputUtxoMerkleTreePubkeys: PublicKey[],
  ): UtxoSerde {
    if (utxosToAdd.length === 0) return this;

    const utxos: OutUtxoSerializableTuple[] = [];
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
          : this.pubkeyArray.findIndex((pubkey) => pubkey.equals(utxo.owner)) >=
              0
            ? this.pubkeyArray.findIndex((pubkey) =>
                pubkey.equals(utxo.owner),
              ) + accounts.length
            : this.pubkeyArray.push(utxo.owner) - 1 + accounts.length;
      const lamportsIndex = this.u64Array.findIndex((l) => l === utxo.lamports);
      const lamports =
        lamportsIndex >= 0
          ? lamportsIndex
          : this.u64Array.push(utxo.lamports) - 1;

      const data = utxo.data
        ? serializeTlv(utxo.data, this.pubkeyArray, accounts)
        : null;

      const outputUtxoSerializable: OutUtxoSerializable_IdlType = {
        owner,
        lamports: lamports,
        data,
        address: utxo.address,
      };

      // Calc index for merkle tree pubkey
      let merkleTreeIndex = merkleTreeIndices.get(
        outputUtxoMerkleTreePubkeys[i].toString(),
      );
      if (merkleTreeIndex === undefined) {
        merkleTreeIndex = remainingAccountsIndices.get(
          outputUtxoMerkleTreePubkeys[i].toString(),
        );
        if (merkleTreeIndex === undefined) {
          merkleTreeIndex =
            remainingAccountsIndices.size + merkleTreeIndices.size;
          merkleTreeIndices.set(
            outputUtxoMerkleTreePubkeys[i].toString(),
            merkleTreeIndex,
          );
        }
      }

      utxos.push({
        outUtxoSerializable: outputUtxoSerializable,
        indexMtAccount: merkleTreeIndex,
      });
    });

    // Extend SerializedUtxos
    this.outUtxos.push(...utxos);
    return this;
  }

  async deserializeInputUtxos(
    hasher: LightWasm,
    accounts: PublicKey[],
    merkleTreeAccounts: PublicKey[],
    nullifierQueues: PublicKey[],
  ): Promise<UtxoWithMerkleContext[]> {
    const inUtxos: UtxoWithMerkleContext[] = [];

    this.inUtxos.forEach(async (inputUtxoSerializableTuple, i) => {
      const inputUtxo = inputUtxoSerializableTuple.inUtxoSerializable;

      // resolve owner
      const owner =
        inputUtxo.owner < accounts.length
          ? accounts[inputUtxo.owner]
          : this.pubkeyArray[inputUtxo.owner - accounts.length];

      // resolve lamports
      const lamports = this.u64Array[inputUtxo.lamports];

      // resolve data
      const data = inputUtxo.data
        ? deserializeTlv(inputUtxo.data, [...accounts, ...this.pubkeyArray])
        : undefined;

      // reconstruct inputUtxo
      const utxo = createUtxo(owner, lamports, data);
      const utxoHash = await createUtxoHash(
        hasher,
        utxo,
        merkleTreeAccounts[inputUtxoSerializableTuple.indexMtAccount],
        inputUtxo.leafIndex,
      );
      const utxoWithMerkleContext = addMerkleContextToUtxo(
        utxo,
        utxoHash,
        merkleTreeAccounts[inputUtxoSerializableTuple.indexMtAccount],
        inputUtxo.leafIndex,
        nullifierQueues[inputUtxoSerializableTuple.indexNullifierArrayAccount],
      );

      inUtxos.push(utxoWithMerkleContext);
    });

    return inUtxos;
  }

  deserializeOutputUtxos(accounts: PublicKey[]): [Utxo, number][] {
    const outUtxos: [Utxo, number][] = [];

    for (const outputUtxoSerializableTuple of this.outUtxos) {
      // Resolve owner
      const owner =
        outputUtxoSerializableTuple.outUtxoSerializable.owner < accounts.length
          ? accounts[outputUtxoSerializableTuple.outUtxoSerializable.owner]
          : this.pubkeyArray[
              outputUtxoSerializableTuple.outUtxoSerializable.owner -
                accounts.length
            ];

      // Resolve lamports
      const lamports =
        this.u64Array[outputUtxoSerializableTuple.outUtxoSerializable.lamports];

      // Resolve data
      const data = outputUtxoSerializableTuple.outUtxoSerializable.data
        ? deserializeTlv(outputUtxoSerializableTuple.outUtxoSerializable.data, [
            ...accounts,
            ...this.pubkeyArray,
          ])
        : undefined;

      // Reconstruct Utxo
      const utxo = createUtxo(owner, lamports, data);

      outUtxos.push([utxo, outputUtxoSerializableTuple.indexMtAccount]);
    }

    return outUtxos;
  }
}

/// TODO: bunch of redundant conversions. optimize.
/** Computes unique utxo value from merkleTree, leafIndex */
const computeBlinding = async (
  hasher: LightWasm,
  merkleTreePublicKey: PublicKey,
  leafIndex: BN,
): Promise<BN254> => {
  /// ensure <254-bit
  const mtHash = await hashToBn254FieldSizeLe(merkleTreePublicKey.toBuffer());
  if (!mtHash) throw new Error('Failed to hash merkle tree public key');

  const mtPubkeyDecStr = bufToDecStr(mtHash[0]);
  const leafIndexDecStr = leafIndex.toString();

  const hashStr = hasher.poseidonHashString([mtPubkeyDecStr, leafIndexDecStr]);
  return createBN254(hashStr);
};

// TODO: add unit tests!
/**
 * Hashes a UTXO preimage. Hash inputs: owner, blinding(merkleTree,leafIndex),
 * lamports, tlvDataHash
 *
 * async for browser crypto.digest support */
export async function createUtxoHash(
  hasher: LightWasm,
  utxo: Utxo,
  merkleTree: PublicKey,
  leafIndex: number | BN,
): Promise<BN254> {
  const { owner, lamports, data } = utxo;
  /// hash all tlv elements into a single hash
  const tlvDataHash = computeTlvDataHash(data, hasher);

  /// ensure <254-bit
  const ownerHash = await hashToBn254FieldSizeLe(owner.toBuffer());
  if (!ownerHash) throw new Error('Failed to hash owner public key');
  const ownerDecStr = bufToDecStr(ownerHash[0]);
  const lamportsDecStr = lamports.toString();

  const blindingDecStr = (
    await computeBlinding(hasher, merkleTree, bn(leafIndex))
  ).toString();

  const hash = hasher.poseidonHashString([
    ownerDecStr,
    blindingDecStr,
    lamportsDecStr,
    tlvDataHash,
  ]);

  return createBN254(hash);
}

export function computeTlvDataHash(
  data: Tlv_IdlType | null,
  hasher: LightWasm,
) {
  const hash = data
    ? hasher.poseidonHashString(
        data.tlvElements.map((d: TlvDataElement_IdlType) => bn(d.dataHash)),
      )
    : bn(0).toString();

  return hash;
}

// let tlv_data_hash = match &self.data {
//   Some(data) => {
//       let hashes = data
//           .tlv_elements
//           .iter()
//           .map(|d| d.data_hash.as_slice())
//           .collect::<Vec<&[u8]>>();
//       Poseidon::hashv(hashes.as_slice()).unwrap()
//   }
//   None => [0u8; 32],
// };
