import {AnchorProvider, Idl, Program} from "@coral-xyz/anchor";
import {PublicKey, SystemProgram} from "@solana/web3.js";
import {
    TransactionError,
    TransactionErrorCode,
    TransactionParametersError,
    TransactionParametersErrorCode
} from "../errors";
import {OutUtxo, Utxo} from "../utxo/utxo-types";
import {PlaceHolderTData, ProgramOutUtxo, ProgramUtxo} from "../utxo/program-utxo-types";
import {hashAndTruncateToCircuit, stringifyAssetsToCircuitInput} from "../utils/hash-utils";
import {BN_0, N_ASSET_PUBKEYS} from "../constants";

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


export function getAssetPubkeys(
    inputUtxos?: (Utxo | ProgramUtxo<PlaceHolderTData>)[],
    outputUtxos?: (OutUtxo | ProgramOutUtxo<PlaceHolderTData>)[],
): { assetPubkeysCircuit: string[]; assetPubkeys: PublicKey[] } {
    const assetPubkeysCircuit: string[] = [
        hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    ];

    const assetPubkeys: PublicKey[] = [SystemProgram.programId];

    const processUtxos = (
        utxos: (
            | Utxo
            | ProgramUtxo<PlaceHolderTData>
            | OutUtxo
            | ProgramOutUtxo<PlaceHolderTData>
            )[],
    ) => {
        utxos.map((utxo) => {
            const splAssetCircuit = stringifyAssetsToCircuitInput(
                utxo.assets,
            )[1].toString();
            if (
                !assetPubkeysCircuit.includes(splAssetCircuit) &&
                splAssetCircuit != "0"
            ) {
                assetPubkeysCircuit.push(splAssetCircuit);
                assetPubkeys.push(utxo.assets[1]);
            }
        });
    };

    if (inputUtxos) processUtxos(inputUtxos);
    if (outputUtxos) processUtxos(outputUtxos);

    if (
        (!inputUtxos && !outputUtxos) ||
        (inputUtxos?.length == 0 && outputUtxos?.length == 0)
    ) {
        throw new TransactionError(
            TransactionErrorCode.NO_UTXOS_PROVIDED,
            "getAssetPubkeys",
            "No input or output utxos provided.",
        );
    }

    // TODO: test this better
    // if (assetPubkeys.length > params?.verifier.config.out) {
    //   throw new TransactionError(
    //     TransactionErrorCode.EXCEEDED_MAX_ASSETS,
    //     "getAssetPubkeys",
    //     `Utxos contain too many different assets ${params?.verifier.config.out} > max allowed: ${N_ASSET_PUBKEYS}`,
    //   );
    // }

    if (assetPubkeys.length > N_ASSET_PUBKEYS) {
        throw new TransactionError(
            TransactionErrorCode.EXCEEDED_MAX_ASSETS,
            "getAssetPubkeys",
            `Utxos contain too many different assets ${assetPubkeys.length} > max allowed: ${N_ASSET_PUBKEYS}`,
        );
    }

    while (assetPubkeysCircuit.length < N_ASSET_PUBKEYS) {
        /// FIX: should be truncated?
        assetPubkeysCircuit.push(BN_0.toString());
        assetPubkeys.push(SystemProgram.programId);
    }

    return { assetPubkeysCircuit, assetPubkeys };
}
