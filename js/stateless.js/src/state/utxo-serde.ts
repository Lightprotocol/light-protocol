import { Keypair, PublicKey } from '@solana/web3.js';
import { LightWasm, WasmFactory } from '@lightprotocol/hasher.rs';
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
import { beforeAll } from 'vitest';
import { uint8Array } from '@metaplex-foundation/beet';

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
/// TODO: add unit tests
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

  const hashStr = hasher.poseidonHashBN([mtPubkeyDecStr, leafIndexDecStr]);
  return hashStr;
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
  _merkleTree: PublicKey,
  _leafIndex: number | BN,
): Promise<BN254> {
  const { owner, lamports, data } = utxo;

  /// hash all tlv elements into a single hash
  const tlvDataHash = computeTlvDataHash(data, hasher);
  /// ensure <254-bit
  const ownerHash = await hashToBn254FieldSizeLe(owner.toBuffer());
  if (!ownerHash) throw new Error('Failed to hash owner public key');
  const ownerDecStr = bufToDecStr(ownerHash[0]);
  const lamportsDecStr = lamports.toString();

  // FIXME: figure why it gets the wrong index
  // const blind = await computeBlinding(hasher, merkleTree, bn(leafIndex));
  // const blindingDecStr = blind.toString();
  //@ts-ignore
  // if (blindingDecStr !== bn(utxo.blinding).toString()) {
  // console.log(
  //   //@ts-ignore
  //   `Blinding mismatch ${blindingDecStr} !== ${bn(utxo.blinding).toString()}`,
  // );
  // }

  const hash = hasher.poseidonHashBN([
    ownerDecStr,
    //@ts-ignore
    bn(utxo.blinding).toString(),
    lamportsDecStr,
    tlvDataHash.toString(),
  ]);

  return createBN254(hash);
}

export function computeTlvDataHash(
  data: Tlv_IdlType | null,
  hasher: LightWasm,
): BN {
  const hash = data
    ? hasher.poseidonHashBN(
        data.tlvElements.map((d: TlvDataElement_IdlType) => bn(d.dataHash)),
      )
    : bn(0);

  return hash;
}

//@ts-ignore
if (import.meta.vitest) {
  const owner = new PublicKey('9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE');

  const blinding = [
    3, 23, 116, 190, 161, 85, 183, 105, 2, 210, 96, 171, 251, 35, 230, 70, 184,
    162, 76, 17, 34, 148, 163, 126, 54, 92, 38, 29, 25, 135, 147, 44,
  ];
  const lamports = bn(0);
  const address = null;
  const data: Tlv_IdlType = {
    tlvElements: [
      {
        discriminator: [2, 0, 0, 0, 0, 0, 0, 0],
        owner,
        data: Uint8Array.from([
          // was buf in
          185, 99, 233, 139, 233, 54, 110, 239, 130, 16, 253, 78, 46, 210, 110,
          241, 63, 35, 100, 98, 171, 164, 116, 59, 163, 104, 7, 62, 220, 50,
          192, 92, 154, 42, 164, 131, 114, 72, 61, 70, 40, 220, 171, 100, 231,
          0, 42, 35, 249, 7, 159, 126, 160, 250, 184, 187, 190, 120, 5, 31, 21,
          130, 70, 233, 100, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
          0,
        ]),
        dataHash: [
          5, 85, 70, 244, 68, 63, 197, 38, 53, 63, 214, 45, 142, 104, 176, 219,
          200, 164, 188, 116, 89, 128, 222, 52, 31, 139, 72, 210, 150, 54, 245,
          162,
        ],
      },
    ],
  };

  const tlvDataHash = [
    37, 111, 121, 76, 74, 33, 21, 53, 189, 124, 233, 254, 147, 209, 178, 120,
    146, 115, 230, 159, 132, 45, 37, 211, 28, 32, 34, 54, 136, 51, 200, 168,
  ];

  const utxoHash = [
    38, 142, 20, 124, 40, 106, 29, 108, 182, 215, 87, 162, 188, 117, 223, 63,
    117, 137, 12, 66, 236, 97, 48, 17, 195, 13, 5, 86, 115, 203, 208, 61,
  ];

  const merkletree = new PublicKey(
    '5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W',
  );
  const leafIndex = bn(0);

  //@ts-ignore
  const { it, expect } = import.meta.vitest;
  let hasher: LightWasm;
  beforeAll(async () => {
    hasher = await WasmFactory.getInstance();
  });
  it.only('should compute tlvdatahash', async () => {
    const testTlvDataHash = computeTlvDataHash(data, hasher);
    expect(testTlvDataHash.eq(bn(tlvDataHash))).toBe(true);
  });

  it.only('should compute blinding ', async () => {
    const hasher = await WasmFactory.getInstance();

    const testBlinding = await computeBlinding(hasher, merkletree, leafIndex);
    expect(testBlinding.eq(bn(blinding))).toBe(true);
  });

  it.only('should compute utxo hash', async () => {
    const testUtxoHash = await createUtxoHash(
      hasher,
      { owner, lamports, address, data },
      merkletree,
      leafIndex,
    );
    expect(testUtxoHash.eq(bn(utxoHash))).toBe(true);
  });

  const blinding2 = [
    1, 30, 61, 100, 35, 25, 68, 223, 106, 158, 239, 247, 188, 144, 184, 248, 31,
    111, 90, 220, 101, 207, 94, 194, 63, 167, 164, 211, 151, 92, 215, 174,
  ];
  const leafIndex2 = bn(2);

  const data2: Tlv_IdlType = {
    tlvElements: [
      {
        discriminator: [2, 0, 0, 0, 0, 0, 0, 0],
        owner,
        data: Uint8Array.from([
          185, 99, 233, 139, 233, 54, 110, 239, 130, 16, 253, 78, 46, 210, 110,
          241, 63, 35, 100, 98, 171, 164, 116, 59, 163, 104, 7, 62, 220, 50,
          192, 92, 154, 42, 164, 131, 114, 72, 61, 70, 40, 220, 171, 100, 231,
          0, 42, 35, 249, 7, 159, 126, 160, 250, 184, 187, 190, 120, 5, 31, 21,
          130, 70, 233, 30, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
          0,
        ]),
        dataHash: [
          39, 140, 59, 146, 9, 118, 104, 254, 164, 57, 70, 83, 181, 36, 85, 132,
          236, 119, 85, 249, 111, 150, 15, 20, 250, 217, 75, 178, 1, 242, 251,
          196,
        ],
      },
    ],
  };

  const tlvDataHash2 = [
    9, 95, 186, 23, 155, 157, 156, 133, 95, 195, 3, 255, 191, 113, 30, 190, 223,
    154, 224, 145, 248, 244, 234, 194, 27, 95, 92, 7, 114, 232, 179, 41,
  ];
  const utxoHash2 = [
    13, 248, 196, 200, 227, 65, 162, 129, 98, 253, 126, 229, 111, 93, 94, 168,
    73, 37, 131, 204, 235, 129, 118, 17, 82, 191, 169, 227, 21, 177, 247, 51,
  ];

  it.only('should compute tlvdatahash2', async () => {
    const testTlvDataHash = computeTlvDataHash(data2, hasher);
    expect(testTlvDataHash.eq(bn(tlvDataHash2))).toBe(true);
  });

  it.only('should compute blinding2 ', async () => {
    const hasher = await WasmFactory.getInstance();

    const testBlinding = await computeBlinding(hasher, merkletree, leafIndex2);
    expect(testBlinding.eq(bn(blinding2))).toBe(true);
  });

  it('should compute utxo hash2', async () => {
    const testUtxoHash = await createUtxoHash(
      hasher,
      { owner, lamports, address, data: data2 },
      merkletree,
      leafIndex2,
    );
    expect(testUtxoHash.toArray()).toEqual(utxoHash2);
  });
}
