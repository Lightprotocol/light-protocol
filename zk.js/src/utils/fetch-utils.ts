import {BN} from "@coral-xyz/anchor";
import {PublicKey} from "@solana/web3.js";

export function fetchAssetByIdLookUp(
    assetIndex: BN,
    assetLookupTable: string[],
): PublicKey {
    return new PublicKey(assetLookupTable[assetIndex.toNumber()]);
}