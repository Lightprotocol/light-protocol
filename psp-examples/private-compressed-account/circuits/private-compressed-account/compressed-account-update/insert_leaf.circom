pragma circom 2.1.4;
include "../../../node_modules/circomlib/circuits/poseidon.circom";
include "../../../node_modules/@lightprotocol/circuit-lib.circom/src/merkle-tree/merkleProof.circom";
include "../../../node_modules/@lightprotocol/circuit-lib.circom/src/light-utils/keypair.circom";
include "../../../node_modules/circomlib/circuits/gates.circom";
include "../../../node_modules/circomlib/circuits/comparators.circom";
include "./merkleTreeUpdater.circom";


/**
* Updates Merkle tree
*
*/
template insert_leaf( levels) {
    signal input updatedRoot;
    signal input leaf; // The leaf at index
    signal input subTrees[levels];
    signal input newSubTrees[levels];
    signal input pathIndices;
    signal input zeroValues[levels]; // Initialized value, typically hash of 0
    signal input sibling;

    signal input subTreeHash;
    signal input newSubTreeHash;
    component merkleTreeUpdater = MerkleUpdateProof(levels);
    merkleTreeUpdater.leaf <== leaf; // The leaf at index
    merkleTreeUpdater.subTrees <== subTrees;
    merkleTreeUpdater.pathIndices <== pathIndices;
    merkleTreeUpdater.zeroValues <== zeroValues; // Initialized value, typically hash of 0
    merkleTreeUpdater.sibling <== sibling;
    merkleTreeUpdater.newSubTrees <== newSubTrees;

    updatedRoot === merkleTreeUpdater.newRoot;
    subTreeHash === merkleTreeUpdater.subTreeHash;
    newSubTreeHash === merkleTreeUpdater.newSubTreeHash;
}
