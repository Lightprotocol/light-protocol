/// <reference types="node" />
import { VerifierProgramTwoIdl } from "../idls/verifier_program_two";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
export declare class VerifierTwo implements Verifier {
    verifierProgram: Program<VerifierProgramTwoIdl>;
    wtnsGenPath: String;
    zkeyPath: String;
    calculateWtns: NodeRequire;
    registeredVerifierPda: PublicKey;
    nrPublicInputs: number;
    constructor();
    parsePublicInputsFromArray(transaction: Transaction): PublicInputs;
    sendTransaction(insert: Boolean): Promise<any>;
}
