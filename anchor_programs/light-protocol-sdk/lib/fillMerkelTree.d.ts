import MerkleTree from "./merkelTree";
import Utxo from "./utxos";
export declare const fillMerkelTree: (inputUtxos: Utxo[], merkelTree: MerkleTree) => {
    inputMerklePathIndices: (number | null)[];
    inputMerklePathElements: (number[] | Object[])[];
};
