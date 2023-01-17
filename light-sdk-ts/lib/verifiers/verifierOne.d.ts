/// <reference types="node" />
import { VerifierProgramOneIdl } from "../idls/verifier_program_one";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierOne implements Verifier {
    verifierProgram: Program<VerifierProgramOneIdl>;
    wtnsGenPath: String;
    zkeyPath: String;
    calculateWtns: NodeRequire;
    config: {
        in: number;
        out: number;
    };
    instructions?: anchor.web3.TransactionInstruction[];
    constructor();
    parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
    getInstructions(transaction: Transaction): Promise<anchor.web3.TransactionInstruction[]>;
}
