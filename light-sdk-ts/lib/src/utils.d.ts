/// <reference types="bn.js" />
import { BN } from "@project-serum/anchor";
import { Connection, PublicKey } from '@solana/web3.js';
export declare function hashAndTruncateToCircuit(data: Uint8Array): BN;
export declare function getAssetLookUpId({ connection, asset }: {
    asset: PublicKey;
    connection: Connection;
}): Promise<any>;
export declare function fetchAssetByIdLookUp({ assetIndex }: {
    assetIndex: BN;
}): PublicKey;
