// import { PublicKey } from '@solana/web3.js';
// import { LightWasm, WasmFactory } from '@lightprotocol/account.rs';
// import { Utxo } from './utxo';
// import { bufToDecStr, hashToBn254FieldSizeLe } from '../utils/conversion';
// import { BN254, bn, createBN254 } from './BN254';
// import { BN } from '@coral-xyz/anchor';
// import { beforeAll } from 'vitest';

// /// TODO: bunch of redundant conversions. optimize.
// /// TODO: add unit tests
// /** Computes unique utxo value from merkleTree, leafIndex */
// const computeBlinding = async (
//     hasher: LightWasm,
//     merkleTreePublicKey: PublicKey,
//     leafIndex: BN,
// ): Promise<BN254> => {
//     /// ensure <254-bit
//     const mtHash = await hashToBn254FieldSizeLe(merkleTreePublicKey.toBuffer());
//     if (!mtHash) throw new Error('Failed to hash merkle tree public key');

//     const mtPubkeyDecStr = bufToDecStr(mtHash[0]);
//     const leafIndexDecStr = leafIndex.toString();

//     const hashStr = hasher.poseidonHashBN([mtPubkeyDecStr, leafIndexDecStr]);
//     return hashStr;
// };

// // TODO: add unit tests!
// /**
//  * Hashes a UTXO preimage. Hash inputs: owner, blinding(merkleTree,leafIndex),
//  * lamports, tlvDataHash
//  *
//  * async for browser crypto.digest support */
// export async function createUtxoHash(
//     hasher: LightWasm,
//     utxo: Utxo,
//     _merkleTree: PublicKey,
//     _leafIndex: number | BN,
// ): Promise<BN254> {
//     const { owner, lamports, data } = utxo;

//     /// hash all tlv elements into a single hash
//     const tlvDataHash = computeTlvDataHash(data, hasher);
//     /// ensure <254-bit
//     const ownerHash = await hashToBn254FieldSizeLe(owner.toBuffer());
//     if (!ownerHash) throw new Error('Failed to hash owner public key');
//     const ownerDecStr = bufToDecStr(ownerHash[0]);
//     const lamportsDecStr = lamports.toString();

//     // FIXME: figure why it gets the wrong index
//     // const blind = await computeBlinding(hasher, merkleTree, bn(leafIndex));
//     // const blindingDecStr = blind.toString();
//     //@ts-ignore
//     // if (blindingDecStr !== bn(utxo.blinding).toString()) {
//     // console.log(
//     //   //@ts-ignore
//     //   `Blinding mismatch ${blindingDecStr} !== ${bn(utxo.blinding).toString()}`,
//     // );
//     // }

//     const hash = hasher.poseidonHashBN([
//         ownerDecStr,
//         //@ts-ignore
//         bn(utxo.blinding).toString(),
//         lamportsDecStr,
//         tlvDataHash.toString(),
//     ]);

//     return createBN254(hash);
// }

// export function computeTlvDataHash(
//     data: Tlv_IdlType | null,
//     hasher: LightWasm,
// ): BN {
//     const hash = data
//         ? hasher.poseidonHashBN(
//               data.tlvElements.map((d: TlvDataElement_IdlType) =>
//                   bn(d.dataHash),
//               ),
//           )
//         : bn(0);

//     return hash;
// }

// //@ts-ignore
// if (import.meta.vitest) {
//     const owner = new PublicKey('9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE');

//     const blinding = [
//         3, 23, 116, 190, 161, 85, 183, 105, 2, 210, 96, 171, 251, 35, 230, 70,
//         184, 162, 76, 17, 34, 148, 163, 126, 54, 92, 38, 29, 25, 135, 147, 44,
//     ];
//     const lamports = bn(0);
//     const address = null;
//     const data: Tlv_IdlType = {
//         tlvElements: [
//             {
//                 discriminator: [2, 0, 0, 0, 0, 0, 0, 0],
//                 owner,
//                 data: Uint8Array.from([
//                     // was buf in
//                     185, 99, 233, 139, 233, 54, 110, 239, 130, 16, 253, 78, 46,
//                     210, 110, 241, 63, 35, 100, 98, 171, 164, 116, 59, 163, 104,
//                     7, 62, 220, 50, 192, 92, 154, 42, 164, 131, 114, 72, 61, 70,
//                     40, 220, 171, 100, 231, 0, 42, 35, 249, 7, 159, 126, 160,
//                     250, 184, 187, 190, 120, 5, 31, 21, 130, 70, 233, 100, 0, 0,
//                     0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                 ]),
//                 dataHash: [
//                     5, 85, 70, 244, 68, 63, 197, 38, 53, 63, 214, 45, 142, 104,
//                     176, 219, 200, 164, 188, 116, 89, 128, 222, 52, 31, 139, 72,
//                     210, 150, 54, 245, 162,
//                 ],
//             },
//         ],
//     };

//     const tlvDataHash = [
//         37, 111, 121, 76, 74, 33, 21, 53, 189, 124, 233, 254, 147, 209, 178,
//         120, 146, 115, 230, 159, 132, 45, 37, 211, 28, 32, 34, 54, 136, 51, 200,
//         168,
//     ];

//     const utxoHash = [
//         38, 142, 20, 124, 40, 106, 29, 108, 182, 215, 87, 162, 188, 117, 223,
//         63, 117, 137, 12, 66, 236, 97, 48, 17, 195, 13, 5, 86, 115, 203, 208,
//         61,
//     ];

//     const merkletree = new PublicKey(
//         '5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W',
//     );
//     const leafIndex = bn(0);

//     //@ts-ignore
//     const { it, expect } = import.meta.vitest;
//     let hasher: LightWasm;
//     beforeAll(async () => {
//         hasher = await WasmFactory.getInstance();
//     });
//     it.only('should compute tlvdatahash', async () => {
//         const testTlvDataHash = computeTlvDataHash(data, hasher);
//         expect(testTlvDataHash.eq(bn(tlvDataHash))).toBe(true);
//     });

//     it.only('should compute blinding ', async () => {
//         const hasher = await WasmFactory.getInstance();

//         const testBlinding = await computeBlinding(
//             hasher,
//             merkletree,
//             leafIndex,
//         );
//         expect(testBlinding.eq(bn(blinding))).toBe(true);
//     });

//     it('should compute utxo hash', async () => {
//         const testUtxoHash = await createUtxoHash(
//             hasher,
//             { owner, lamports, address, data },
//             merkletree,
//             leafIndex,
//         );
//         expect(testUtxoHash.eq(bn(utxoHash))).toBe(true);
//     });

//     const blinding2 = [
//         1, 30, 61, 100, 35, 25, 68, 223, 106, 158, 239, 247, 188, 144, 184, 248,
//         31, 111, 90, 220, 101, 207, 94, 194, 63, 167, 164, 211, 151, 92, 215,
//         174,
//     ];
//     const leafIndex2 = bn(2);

//     const data2: Tlv_IdlType = {
//         tlvElements: [
//             {
//                 discriminator: [2, 0, 0, 0, 0, 0, 0, 0],
//                 owner,
//                 data: Uint8Array.from([
//                     185, 99, 233, 139, 233, 54, 110, 239, 130, 16, 253, 78, 46,
//                     210, 110, 241, 63, 35, 100, 98, 171, 164, 116, 59, 163, 104,
//                     7, 62, 220, 50, 192, 92, 154, 42, 164, 131, 114, 72, 61, 70,
//                     40, 220, 171, 100, 231, 0, 42, 35, 249, 7, 159, 126, 160,
//                     250, 184, 187, 190, 120, 5, 31, 21, 130, 70, 233, 30, 0, 0,
//                     0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                 ]),
//                 dataHash: [
//                     39, 140, 59, 146, 9, 118, 104, 254, 164, 57, 70, 83, 181,
//                     36, 85, 132, 236, 119, 85, 249, 111, 150, 15, 20, 250, 217,
//                     75, 178, 1, 242, 251, 196,
//                 ],
//             },
//         ],
//     };

//     const tlvDataHash2 = [
//         9, 95, 186, 23, 155, 157, 156, 133, 95, 195, 3, 255, 191, 113, 30, 190,
//         223, 154, 224, 145, 248, 244, 234, 194, 27, 95, 92, 7, 114, 232, 179,
//         41,
//     ];
//     const utxoHash2 = [
//         13, 248, 196, 200, 227, 65, 162, 129, 98, 253, 126, 229, 111, 93, 94,
//         168, 73, 37, 131, 204, 235, 129, 118, 17, 82, 191, 169, 227, 21, 177,
//         247, 51,
//     ];

//     it.only('should compute tlvdatahash2', async () => {
//         const testTlvDataHash = computeTlvDataHash(data2, hasher);
//         expect(testTlvDataHash.eq(bn(tlvDataHash2))).toBe(true);
//     });

//     it.skip('should compute blinding2 ', async () => {
//         const hasher = await WasmFactory.getInstance();

//         const testBlinding = await computeBlinding(
//             hasher,
//             merkletree,
//             leafIndex2,
//         );
//         expect(testBlinding.eq(bn(blinding2))).toBe(true);
//     });

//     it('should compute utxo hash2', async () => {
//         const testUtxoHash = await createUtxoHash(
//             hasher,
//             { owner, lamports, address, data: data2 },
//             merkletree,
//             leafIndex2,
//         );
//         expect(testUtxoHash.toArray()).toEqual(utxoHash2);
//     });
// }
