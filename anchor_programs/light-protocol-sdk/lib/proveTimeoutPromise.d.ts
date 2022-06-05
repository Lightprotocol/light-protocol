import Utxo from "./utxos";
export declare const proofTimeoutPromise: (input: any, inputUtxos: Utxo[]) => Promise<{
    data: {
        publicInputsBytes: any[];
        proofBytes: any;
    };
}>;
