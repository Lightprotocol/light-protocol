/// <reference types="bn.js" />
import { BN } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
export declare function hashAndTruncateToCircuit(data: Uint8Array): BN;
export declare function getAssetLookUpId({
  connection,
  asset,
}: {
  asset: PublicKey;
  connection: Connection;
}): Promise<any>;
export declare const assetLookupTable: PublicKey[];
export declare function getAssetIndex(assetPubkey: PublicKey): BN;
export declare function fetchAssetByIdLookUp(assetIndex: BN): PublicKey;
