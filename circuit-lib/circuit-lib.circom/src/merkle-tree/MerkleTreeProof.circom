pragma circom 2.1.4;

include "../../node_modules/circomlib/circuits/poseidon.circom";
include "../../node_modules/circomlib/circuits/comparators.circom";
include "../../node_modules/circomlib/circuits/bitify.circom";
include "../../node_modules/circomlib/circuits/switcher.circom";

template MerkleTreeProof(levels, numberOfUTXOs) {
    signal input root[numberOfUTXOs];
    signal input leaf[numberOfUTXOs];
    signal input inPathElements[numberOfUTXOs][levels];
    signal input inPathIndices[numberOfUTXOs];

    component inTree[numberOfUTXOs];
    for (var i = 0; i < numberOfUTXOs; i++) {
        inTree[i] = MerkleProof(levels);
        inTree[i].leaf <== leaf[i];
        inTree[i].pathIndices <== inPathIndices[i];
        inTree[i].pathElements <== inPathElements[i];
        inTree[i].root === root[i];
    }
}

// Verifies that merkle proof is correct for given merkle root and a leaf
// pathIndices bits is an array of 0/1 selectors telling whether given pathElement is on the left or right side of merkle path
template MerkleProof(levels) {
    signal input leaf;
    signal input pathElements[levels];
    signal input pathIndices;
    signal output root;

    component switcher[levels];
    component hasher[levels];

    component indexBits = Num2Bits(levels);
    indexBits.in <== pathIndices;

    for (var i = 0; i < levels; i++) {
        switcher[i] = Switcher();
        switcher[i].L <== i == 0 ? leaf : hasher[i - 1].out;
        switcher[i].R <== pathElements[i];
        switcher[i].sel <== indexBits.out[i];

        hasher[i] = Poseidon(2);
        hasher[i].inputs[0] <== switcher[i].outL;
        hasher[i].inputs[1] <== switcher[i].outR;
    }
    root <== hasher[levels - 1].out;
}
