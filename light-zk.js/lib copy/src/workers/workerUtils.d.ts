import { UtxoBatch } from "wallet";
import { UtxoBytes } from "utxo";
export declare function callDecryptUtxoBytesWorker(params: {
    encBytesArray: UtxoBatch[];
    compressed: boolean;
    aesSecret: Uint8Array | undefined;
    asymSecret: Uint8Array | undefined;
    merkleTreePdaPublicKeyString: string;
}): Promise<UtxoBytes[]>;
