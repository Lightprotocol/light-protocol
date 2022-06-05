import { BigNumber } from 'ethers';
import { Action } from './enums';
import MerkleTree from './merkelTree';
import Utxo from './utxos';
declare const nacl: any;
export declare const getProof: (inputUtxos: Utxo[] | undefined, outputUtxos: Utxo[] | undefined, merkelTree: MerkleTree, externalAmountBigNumber: BigNumber, relayerFee: any, recipient: string, relayer: string, action: Action, encryptionKeypair: nacl.BoxKeyPair) => Promise<{
    data: {
        extAmount: Uint8Array;
        externalAmountBigNumber: BigNumber;
        extDataBytes: Uint8Array;
        publicInputsBytes: any[];
        proofBytes: any;
        encryptedOutputs: Uint8Array[];
    };
}>;
export {};
