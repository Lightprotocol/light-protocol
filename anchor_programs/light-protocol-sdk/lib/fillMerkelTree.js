"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.fillMerkelTree = void 0;
const toFixedHex_1 = require("./utils/toFixedHex");
const fillMerkelTree = (inputUtxos, merkelTree) => {
    let inputMerklePathIndices = [];
    let inputMerklePathElements = [];
    /// if the input utxo has an amount bigger than 0 and it has an valid index add it to the indices of the merkel tree
    /// also push the path to the leaf
    /// else push a 0 to the indices
    /// and fill the path to the leaf with 0s 
    for (const inputUtxo of inputUtxos) {
        if (inputUtxo.amount > 0) {
            inputUtxo.index = merkelTree.indexOf((0, toFixedHex_1.toFixedHex)(inputUtxo.getCommitment()));
            if (inputUtxo.index) {
                if (inputUtxo.index < 0) {
                    throw new Error(`Input commitment ${(0, toFixedHex_1.toFixedHex)(inputUtxo.getCommitment())} was not found`);
                }
                inputMerklePathIndices.push(inputUtxo.index);
                inputMerklePathElements.push(merkelTree.path(inputUtxo.index).pathElements);
            }
        }
        else {
            inputMerklePathIndices.push(0);
            inputMerklePathElements.push(new Array(merkelTree.levels).fill(0));
        }
    }
    return { inputMerklePathIndices, inputMerklePathElements };
};
exports.fillMerkelTree = fillMerkelTree;
