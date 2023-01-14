/// <reference types="node" />
export * from "./verifierOne";
export * from "./verifierZero";
export * from "./verifierTwo";
import { Program, web3 } from "@coral-xyz/anchor";
import { Transaction } from "../transaction";
export interface PublicInputs {
    root: Array<Number>;
    publicAmount: Array<Number>;
    extDataHash: Array<Number>;
    feeAmount: Array<Number>;
    mintPubkey: Array<Number>;
    nullifiers: Array<Uint8Array>;
    leaves: Array<Array<Number>>;
}
export interface PublicInputsCpi {
    root: Array<Number>;
    publicAmount: Array<Number>;
    extDataHash: Array<Number>;
    feeAmount: Array<Number>;
    mintPubkey: Array<Number>;
    verifier: Array<Number>;
    appHash: Array<Number>;
    kycMtRoot: Array<Number>;
    nullifiers: Array<Uint8Array>;
    leaves: Array<Uint8Array>;
}
export interface Verifier {
    verifierProgram: Program<any>;
    wtnsGenPath: String;
    zkeyPath: String;
    calculateWtns: NodeRequire;
    config: {
        in: number;
        out: number;
    };
    instructions?: web3.TransactionInstruction[];
    parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
    getInstructions(transaction: Transaction): Promise<web3.TransactionInstruction[]>;
}
