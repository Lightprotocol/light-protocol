// import { Idl } from '@coral-xyz/anchor';
// //@ts-ignore
// import { Prover } from '@lightprotocol/prover.js';
// import { ProofError, ProofErrorCode } from '../errors';
// import { InclusionProofInputs } from './interface';

// function getProverInstance(
//     verifierIdl: Idl,
//     firstPath: string,
//     circuitName?: string,
// ) {
//     return new Prover(verifierIdl, firstPath, circuitName);
// }

// // TODO: implement types
// export type WasmTester = any;
// export type ParsedProof = any;
// export type ParsedPublicInputs = any;
// export type ParsedPublicInputsObject = any;
// export type AnyError = any;

// export type Proof = {
//     parsedProof: ParsedProof;
//     parsedPublicInputsObject: ParsedPublicInputsObject;
// };

// /// The tx is not yet created at the time of proof generation. Therefore,
// /// we need to call the getProof before requesting a signature. That's ok
// /// because proof generation in the wallet doesn't require a secret!
// export async function getProofInternal({
//     proofInputs,
//     verifierIdl,
//     firstPath,
//     circuitName,
//     getProver = getProverInstance,
//     verify = true,
//     enableLogging,
//     wasmTester,
// }: {
//     proofInputs: InclusionProofInputs;
//     verifierIdl: Idl;
//     firstPath: string;
//     circuitName?: string;
//     getProver?: any;
//     verify?: boolean;
//     enableLogging?: boolean;
//     wasmTester?: WasmTester;
// }): Promise<Proof> {
//     const prover = await getProver(
//         verifierIdl,
//         firstPath,
//         circuitName,
//         wasmTester,
//     );

//     await prover.addProofInputs(proofInputs);

//     /// debug
//     const prefix = `\x1b[37m[${new Date(Date.now()).toISOString()}]\x1b[0m`;
//     const logMsg = `${prefix} Proving ${verifierIdl.name} circuit`;
//     if (enableLogging) console.time(logMsg);

//     let parsedProof: ParsedProof, parsedPublicInputs: ParsedPublicInputs;

//     try {
//         const result = await prover.fullProveAndParse();
//         parsedProof = result.parsedProof;
//         parsedPublicInputs = result.parsedPublicInputs;
//     } catch (error: AnyError) {
//         throw new ProofError(
//             ProofErrorCode.PROOF_GENERATION_FAILED,
//             'getProofInternal',
//             error,
//         );
//     }

//     /// debug
//     if (enableLogging) console.timeEnd(logMsg);
//     if (verify || enableLogging) {
//         const res = await prover.verify();
//         if (!res)
//             throw new ProofError(
//                 ProofErrorCode.INVALID_PROOF,
//                 'getProofInternal',
//             );
//     }

//     const parsedPublicInputsObject: ParsedPublicInputsObject =
//         prover.parsePublicInputsFromArray(parsedPublicInputs);

//     return { parsedProof, parsedPublicInputsObject };
// }
