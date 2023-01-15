/// <reference types="node" />
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { VerifierProgramZeroIdl } from "../idls/verifier_program_zero";
export declare class VerifierZero implements Verifier {
    verifierProgram: Program<VerifierProgramZeroIdl>;
    wtnsGenPath: String;
    zkeyPath: String;
    calculateWtns: NodeRequire;
    config: {
        in: number;
        out: number;
    };
    instructions?: anchor.web3.TransactionInstruction[];
    constructor();
    parsePublicInputsFromArray(publicInputsBytes: Uint8Array): PublicInputs;
    getInstructions(transaction: Transaction): Promise<anchor.web3.TransactionInstruction[]>;
}
