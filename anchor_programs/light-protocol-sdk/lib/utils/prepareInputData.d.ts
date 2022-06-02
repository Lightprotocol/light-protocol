import { BigNumber } from "ethers";
import MerkleTree from "../merkelTree";
import Utxo from "../utxos";
export declare const prepareInputData: (merkelTree: MerkleTree, inputUtxos: Utxo[], outputUtxos: Utxo[], inputMerklePathIndices: (number | null)[], inputMerklePathElements: (Object[] | number[])[], externalAmountBigNumber: BigNumber, relayerFee: any, extDataHash: BigNumber) => {
    root: any;
    inputNullifier: (BigNumber | null)[];
    outputCommitment: (BigNumber | null)[];
    publicAmount: string;
    extDataHash: BigNumber;
    inAmount: (number | BigNumber)[];
    inPrivateKey: string[];
    inBlinding: BigNumber[];
    inPathIndices: (number | null)[];
    inPathElements: (number[] | Object[])[];
    outAmount: (number | BigNumber)[];
    outBlinding: BigNumber[];
    outPubkey: any[];
};
