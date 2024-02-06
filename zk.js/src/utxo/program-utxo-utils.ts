
// TODO: remove verifier index from encrypted utxo data
// TODO: add explicit type to serialized data
import {PlaceHolderTData, ProgramOutUtxo} from "utxo/program-utxo-types";
import {hashAndTruncateToCircuit} from "utils/hash-utils";
import {BN_0, COMPRESSED_UTXO_BYTES_LENGTH} from "../constants";
import {UtxoError, UtxoErrorCode} from "errors";
import {BN, BorshAccountsCoder} from "@coral-xyz/anchor";
import {PublicKey} from "@solana/web3.js";

/** Parse a program-owned utxo to bytes */
export async function programOutUtxoToBytes(
    outUtxo: ProgramOutUtxo<PlaceHolderTData>,
    assetLookupTable: string[],
    compressed: boolean = false,
): Promise<Uint8Array> {
    const serializeObject = {
        ...outUtxo,
        ...outUtxo.data,
        /// TODO: fix idl naming congruence
        appDataHash: outUtxo.dataHash,
        /// FIX: check if we need this for programutxos anymore
        accountCompressionPublicKey: hashAndTruncateToCircuit(
            outUtxo.owner.toBytes(),
        ),
        accountEncryptionPublicKey:
            outUtxo.encryptionPublicKey ?? new Uint8Array(32).fill(0),
        verifierAddressIndex: BN_0,
        splAssetIndex: getSplAssetLookupTableIndex(
            outUtxo.assets[1],
            assetLookupTable,
        ),
    };
    if (serializeObject.splAssetIndex.toString() === "-1") {
        throw new UtxoError(
            UtxoErrorCode.ASSET_NOT_FOUND,
            "outUtxoToBytes",
            `asset pubkey ${serializeObject.assets[1]}, not found in lookup table`,
        );
    }
    const coder = new BorshAccountsCoder(outUtxo.ownerIdl);
    const serializedData = await coder.encode(
        outUtxo.type + "OutUtxo",
        serializeObject,
    );

    // Compressed serialization does not store the account since for an encrypted utxo
    // we assume that the user who is able to decrypt the utxo knows the corresponding account.
    return compressed
        ? serializedData.subarray(0, COMPRESSED_UTXO_BYTES_LENGTH)
        : serializedData;
}


const getSplAssetLookupTableIndex = (
    asset: PublicKey,
    assetLookupTable: string[],
): BN => {
    const index = assetLookupTable.findIndex(
        (base58PublicKey) => base58PublicKey === asset.toBase58(),
    );
    if (index === -1) {
        throw new UtxoError(
            UtxoErrorCode.ASSET_NOT_FOUND,
            "getSplAssetLookupTableIndex",
            `asset pubkey ${asset}, not found in lookup table`,
        );
    }
    return new BN(index);
};
