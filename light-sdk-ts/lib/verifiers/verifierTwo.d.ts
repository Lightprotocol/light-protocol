/// <reference types="node" />
/// <reference types="bn.js" />
import { VerifierProgramTwoIdl } from "../idls/verifier_program_two";
import { Program } from "@coral-xyz/anchor";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { BN } from "@coral-xyz/anchor";
export declare class VerifierTwo implements Verifier {
    verifierProgram: Program<VerifierProgramTwoIdl>;
    wtnsGenPath: String;
    zkeyPath: String;
    calculateWtns: NodeRequire;
    nrPublicInputs: number;
    config: {
        in: number;
        out: number;
    };
    pubkey: BN;
    constructor();
    parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
    initVerifierProgram(): void;
    getInstructions(transaction: Transaction): Promise<any>;
}
