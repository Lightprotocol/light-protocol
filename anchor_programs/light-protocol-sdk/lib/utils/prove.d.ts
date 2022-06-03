import { BigNumber } from 'ethers';
export declare const prove: (input: {
    root: any;
    inputNullifier: (BigNumber | null)[];
    outputCommitment: (BigNumber | null)[];
    publicAmount: string;
    extDataHash: BigNumber;
    inAmount: (number | BigNumber)[];
    inPrivateKey: string[];
    inBlinding: BigNumber[];
    inPathIndices: (number | null)[];
    inPathElements: (Object[] | number[])[];
    outAmount: (number | BigNumber)[];
    outBlinding: BigNumber[];
    outPubkey: any[];
}, keyBasePath: string) => Promise<{
    proofJson: string;
    publicInputsJson: string;
}>;
