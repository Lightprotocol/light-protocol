// TODO: add as refactor into test-rpc.ts and rpc.ts
// async getValidityProof(
//     hashes: BN254[],
//     newAddresses: PublicKey[] = [],
// ): Promise<CompressedProofWithContext> {
//     if (hashes.length === 0 && newAddresses.length === 0) {
//         throw new Error('No hashes or new addresses provided');
//     }
//     if (hashes.length > 0 && newAddresses.length > 0) {
//         throw new Error('Combined proofs not implemented yet');
//     }
//     if (hashes.length > 0 && newAddresses.length === 0) {
//         const { compressedProof, merkleProofsWithContext } =
//             await getInclusionProof(this, hashes);
//         const value: CompressedProofWithContext = {
//             compressedProof,
//             roots: merkleProofsWithContext.map(proof => proof.root),
//             rootIndices: merkleProofsWithContext.map(
//                 proof => proof.rootIndex,
//             ),
//             leafIndices: merkleProofsWithContext.map(
//                 proof => proof.leafIndex,
//             ),
//             leaves: merkleProofsWithContext.map(proof => bn(proof.hash)),
//             merkleTrees: merkleProofsWithContext.map(
//                 proof => proof.merkleTree,
//             ),
//             nullifierQueues: merkleProofsWithContext.map(
//                 proof => proof.nullifierQueue,
//             ),
//         };
//         return value;
//     }
//     if (hashes.length === 0 && newAddresses.length > 0) {
//         const proof = await getExclusionProof(this, newAddresses);
//     }
//     // return await getExclusionProof(this, newAddresses);
// }
// }

// async function getInclusionProof(
// rpc: Rpc,
// hashes: BN254[],
// ): Promise<{
// compressedProof: CompressedProof;
// merkleProofsWithContext: MerkleContextWithMerkleProof[];
// }> {
// /// get merkle proofs
// const merkleProofsWithContext =
//     await rpc.getMultipleCompressedAccountProofs(hashes);

// /// to hex
// const inputs: HexInputsForProver[] = [];
// for (let i = 0; i < merkleProofsWithContext.length; i++) {
//     const input: HexInputsForProver = {
//         root: toHex(merkleProofsWithContext[i].root),
//         pathIndex: merkleProofsWithContext[i].leafIndex,
//         pathElements: merkleProofsWithContext[i].merkleProof.map(hex =>
//             toHex(hex),
//         ),
//         leaf: toHex(bn(merkleProofsWithContext[i].hash)),
//     };
//     inputs.push(input);
// }

// const batchInputs: HexBatchInputsForProver = {
//     'input-compressed-accounts': inputs,
// };
// const inputsData = JSON.stringify(batchInputs);

// const PROOF_URL = `${rpc.proverEndpoint}/prove`;
// const response = await fetch(PROOF_URL, {
//     method: 'POST',
//     headers: {
//         'Content-Type': 'application/json',
//     },
//     body: inputsData,
// });
// if (!response.ok) {
//     throw new Error(`Error fetching proof: ${response.statusText}`);
// }

// // TOOD: add type checks
// const data: any = await response.json();
// const parsed = proofFromJsonStruct(data);
// const compressedProof = negateAndCompressProof(parsed);

// return { compressedProof, merkleProofsWithContext };
// }

// async function getExclusionProof(rpc: Rpc, newAddresses: PublicKey[]) {
// const newAddressesHex = newAddresses.map(address =>
//     toHex(bn(address.toBase58())),
// );
// const inputsData = JSON.stringify({
//     'new-addresses': newAddressesHex,
// });

// const PROOF_URL = `${rpc.proverEndpoint}/prove-exclusion`;
// const response = await fetch(PROOF_URL, {
//     method: 'POST',
//     headers: {
//         'Content-Type': 'application/json',
//     },
//     body: inputsData,
// });
// if (!response.ok) {
//     throw new Error(`Error fetching proof: ${response.statusText}`);
// }

// const data = await response.json();
// const parsed = proofFromJsonStruct(data);
// const compressedProof = negateAndCompressProof(parsed);

// return compressedProof;
// }
