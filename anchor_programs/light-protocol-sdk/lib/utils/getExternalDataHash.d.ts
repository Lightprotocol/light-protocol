import { BigNumber } from 'ethers';
export declare const getExtDataHash: (recipient: string, extAmount: Uint8Array, relayer: string, fee: Uint8Array, merkleTreePubkeyBytes: any, encryptedOutput1: Uint8Array, encryptedOutput2: Uint8Array, nonce1: Uint8Array, nonce2: Uint8Array, senderThrowAwayPubkey1: Uint8Array, senderThrowAwayPubkey2: Uint8Array) => {
    extDataHash: BigNumber;
    extDataBytes: Uint8Array;
};
