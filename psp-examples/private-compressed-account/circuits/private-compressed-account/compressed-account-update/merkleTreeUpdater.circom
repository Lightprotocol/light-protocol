pragma circom 2.1.4;
include "../../node_modules/circomlib/circuits/bitify.circom";
include "../../node_modules/circomlib/circuits/poseidon.circom";
include "../../node_modules/circomlib/circuits/switcher.circom";
include "../../node_modules/circomlib/circuits/gates.circom";

template SelectValue() {
    signal input index;      // 0 or 1
    signal input leaf;   
    signal output out;
    out <== (1 - index) * leaf;
}

template MerkleUpdateProof(levels) {
    signal input leaf; // The leaf to be added at index
    signal input subTrees[levels];
    signal input newSubTrees[levels];
    signal input pathIndices;
    signal input zeroValues[levels]; // Initialized value, typically hash of 0
    signal input sibling;

    signal output subTreeHash;
    signal output newSubTreeHash;
    signal output newRoot; // Updated merkle root after adding the leaf

    component switcher[levels];
    component hasher[levels];

    component indexBits = Num2Bits(levels);
    indexBits.in <== pathIndices;
    component selectFirstValue = SelectValue();
    selectFirstValue.index <== indexBits.out[0];
    selectFirstValue.leaf <== sibling;
    component selectSecondValue = SelectValue();
    selectSecondValue.index <== indexBits.out[0];
    selectSecondValue.leaf <== leaf;

    component checkNewSubTrees[levels];

    component selectZeroValue[levels];
    for (var i = 0; i < levels; i++) {
        switcher[i] = Switcher();
        selectZeroValue[i] = SelectValue();
        selectZeroValue[i].index <== indexBits.out[i];
        selectZeroValue[i].leaf <== zeroValues[i];
        switcher[i].L <== i == 0 ? leaf : hasher[i - 1].out;
        switcher[i].R <== i == 0 ? sibling * indexBits.out[i] + selectZeroValue[i].out : subTrees[i] * indexBits.out[i] + selectZeroValue[i].out;
        switcher[i].sel <== indexBits.out[i];
        hasher[i] = Poseidon(2);
        hasher[i].inputs[0] <== switcher[i].outL;
        hasher[i].inputs[1] <== switcher[i].outR;
        checkNewSubTrees[i] = ForceEqualIfEnabled();
        checkNewSubTrees[i].in[0] <== switcher[i].outL;
        checkNewSubTrees[i].in[1] <== newSubTrees[i];
        checkNewSubTrees[i].enabled <== indexBits.out[i];
    }
    component subTreeHasher = getSubTreeHash(levels, 2, 9);
    subTreeHasher.subTrees <== subTrees;
    component newSubTreeHasher = getSubTreeHash(levels, 2, 9);
    newSubTreeHasher.subTrees <== newSubTrees;
    newSubTreeHash <== newSubTreeHasher.subTreeHash;
    subTreeHash <== subTreeHasher.subTreeHash;
    newRoot <== hasher[levels - 1].out;
}

template getSubTreeHash(levels, hasherLevels, hasherLevelsInternal) {
    signal input subTrees[levels];
    signal output subTreeHash;
    component subTreeHasher = Poseidon(hasherLevels);
    component newSubTreeHasherInternal[hasherLevels];
    var counter = 0;
    for(var i = 0; i < hasherLevels; i++) {
        newSubTreeHasherInternal[i] = Poseidon(hasherLevelsInternal);
        for(var j = 0; j < hasherLevelsInternal; j++) {
            newSubTreeHasherInternal[i].inputs[j] <== subTrees[counter];
            counter++;
        }
        subTreeHasher.inputs[i] <== newSubTreeHasherInternal[i].out;
    }
    subTreeHash <== subTreeHasher.out;
}