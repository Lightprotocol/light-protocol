pragma circom 2.1.4;

include "poseidon.circom";
include "switcher.circom";
include "bitify.circom";
include "comparators.circom";
include "gates.circom";
include "../inclusion-merkle-proof/InclusionMerkleProof.circom";

/*
*

Public inputs:
1. root
2. nonIncludedValue

Private inputs:
1. leafLowerRangeValue (indexedElement.value)
2. leafHigherRangeValue (indexedElement.nextValue)
3. leafIndex (indexedElement.index)
4. merkleProofHashedIndexedElementLeaf
5. indexHashedIndexedElementLeaf

Logic:
leafLowerRangeValue < nonIncludedValue < leafHigherRangeValue;
let leaf = Poseidon(leafLowerRangeValue, leafIndex, leafHigherRangeValue)
let _root = inclusionProof(leaf, merkleProofHashedIndexedElementLeaf, indexHashedIndexedElementLeaf)
assert_eq!(root, _root);
 */

template NonInclusionMerkleProof(levels, numberOfUTXOs) {

    // public:
    signal input root[numberOfUTXOs];
    signal input value[numberOfUTXOs];

    // private:
    signal input leafLowerRangeValue[numberOfUTXOs];
    signal input leafHigherRangeValue[numberOfUTXOs];
    signal input leafIndex[numberOfUTXOs];
    signal input merkleProofHashedIndexedElementLeaf[numberOfUTXOs][levels];
    signal input indexHashedIndexedElementLeaf[numberOfUTXOs];

    component inTree[numberOfUTXOs];
    for (var i = 0; i < numberOfUTXOs; i++) {
        inTree[i] = NonInclusionProof(levels);

        inTree[i].root <== root[i];
        inTree[i].value <== value[i];

        inTree[i].leafLowerRangeValue <== leafLowerRangeValue[i];
        inTree[i].leafHigherRangeValue <== leafHigherRangeValue[i];

        inTree[i].leafIndex <== leafIndex[i];
        inTree[i].merkleProofHashedIndexedElementLeaf <== merkleProofHashedIndexedElementLeaf[i];
        inTree[i].indexHashedIndexedElementLeaf <== indexHashedIndexedElementLeaf[i];
    }
}

template NonInclusionProof(levels) {
        log("NonInclusionProof, levels = ", levels);
        signal input root;
        signal input value;

        signal input leafLowerRangeValue;
        signal input leafHigherRangeValue;
        signal input leafIndex;
        signal input merkleProofHashedIndexedElementLeaf[levels];
        signal input indexHashedIndexedElementLeaf;

        log("NonInclusionProof, checking higherThanLower...");
        // check that leafLowerRangeValue less than notIncludedValue
        component higherThanLower = LessThan(252);
        higherThanLower.in[0] <== leafLowerRangeValue;
        higherThanLower.in[1] <== value;
        signal leafLowerRangeValueLessThanNotIncludedValue <== higherThanLower.out;
        leafLowerRangeValueLessThanNotIncludedValue === 1;


        log("NonInclusionProof, checking lessThanHigher...");
        // check that notIncludedValue less than leafHigherRangeValue
        // TODO: check that value is in Fr(254) on-chain
        component lessThanHigher = LessThan(252);
        lessThanHigher.in[0] <== value;
        lessThanHigher.in[1] <== leafHigherRangeValue;
        signal notIncludedValueLessThanLeafHigherRangeValue <== lessThanHigher.out;
        notIncludedValueLessThanLeafHigherRangeValue === 1;

        
        log("NonInclusionProof, calculating leaf...");
        // Leaf Calculation
        component poseidon = Poseidon(3); // Poseidon with 3 inputs
        poseidon.inputs[0] <== leafLowerRangeValue;
        poseidon.inputs[1] <== leafIndex;
        poseidon.inputs[2] <== leafHigherRangeValue;
        signal leaf <== poseidon.out;

        log("NonInclusionProof, poseidon(", leafLowerRangeValue, ", ", leafIndex, ", ", leafHigherRangeValue, ") = ", leaf);

        // Inclusion Proof Attempt
        component merkleProof = MerkleProof(levels);
        merkleProof.leaf <== leaf;
        merkleProof.pathElements <== merkleProofHashedIndexedElementLeaf;
        merkleProof.pathIndices <== indexHashedIndexedElementLeaf;

        log("NonInclusionProof, root = ", root, ", merkleProof.root = ", merkleProof.root);

        merkleProof.root === root;
}