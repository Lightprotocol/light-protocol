import {AnchorProvider, Idl, Program} from "@coral-xyz/anchor";
import {PublicKey} from "@solana/web3.js";
import {TransactionParametersError, TransactionParametersErrorCode} from "errors";

export function getVerifierProgram(
    verifierIdl: Idl,
    anchorProvider: AnchorProvider,
): Program<Idl> {
    const programId = getVerifierProgramId(verifierIdl);
    return new Program(verifierIdl, programId, anchorProvider);
}

export function getVerifierProgramId(verifierIdl: Idl): PublicKey {
    const programIdObj = verifierIdl.constants!.find(
        (constant) => constant.name === "PROGRAM_ID",
    );
    if (!programIdObj || typeof programIdObj.value !== "string") {
        throw new TransactionParametersError(
            TransactionParametersErrorCode.PROGRAM_ID_CONSTANT_UNDEFINED,
            'PROGRAM_ID constant not found in idl. Example: pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";',
        );
    }

    // Extracting the public key string value from the object and removing quotes.
    const programIdStr = programIdObj.value.slice(1, -1);
    return new PublicKey(programIdStr);
}