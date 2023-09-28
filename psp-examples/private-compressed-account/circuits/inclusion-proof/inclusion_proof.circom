pragma circom 2.1.4;
include "../../node_modules/circomlib/circuits/poseidon.circom";
include "../../node_modules/@lightprotocol/circuit-lib.circom/src/merkle-tree/merkleProof.circom";
include "../../node_modules/circomlib/circuits/comparators.circom";

/**
* Proves the inclusion of a leaf in a Merkle Tree
*
*/
template inclusion_proof( levels) {
    signal input leafPreimage;
    signal input pathElements[levels];
    signal input index;
    signal input root;
    signal input referenceValue;

    component leafHasher = Poseidon(1);
    leafHasher.inputs[0] <== leafPreimage;

    component merkleProof = MerkleProof(levels);
    merkleProof.leaf <== leafHasher.out; // The leaf at index
    merkleProof.pathElements <== pathElements; // The path elements 
    merkleProof.pathIndices <== index; // The index of the leaf

    root === merkleProof.root;

    component greaterThan = GreaterEqThan(64);
    greaterThan.in[0] <== leafPreimage;
    greaterThan.in[1]<== referenceValue;
    greaterThan.out === 1;
}
