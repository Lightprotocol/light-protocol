pragma circom 2.1.4;

include "../non-inclusion-merkle-proof/NonInclusionMerkleProof.circom";
include "../inclusion-merkle-proof/InclusionMerkleProof.circom";

/*
Template `CombinedMerkleProof` is a combination of both an Inclusion Proof and a Non-Inclusion proof.

It takes the following parameters:

1. `levels`: the number of levels for the merkle tree.
2. `numberOfUTXOs`: the number of UTXOs for the inclusion proof.
3. `niNumberOfUTXOs`: the number of UTXOs for the non-inclusion proof.

It includes the following signals:

- Inclusion Proof inputs:
  - `root[numberOfUTXOs]` : public signal input for the root of the merkle tree for each UTXO.
  - `leaf[numberOfUTXOs]` : public signal input for the leaf of the merkle tree for each UTXO.
  - `pathElements[numberOfUTXOs][levels]` : private signal input for the path elements (hashes) for each UTXO.
  - `pathIndices[numberOfUTXOs]` : private signal input for the path indices for each UTXO.

- Non-Inclusion Proof inputs:
  - `niRoot[niNumberOfUTXOs]` : public input signal for the root of the non-included merkle tree for each non-included UTXO.
  - `niValue[niNumberOfUTXOs]` : public input signal indicating the non-included value for each non-included UTXO.
  - `niPathElements[niNumberOfUTXOs][levels]` : private signal input for the path elements for each non-included UTXO.
  - `niPathIndices[niNumberOfUTXOs]` : private signal input for the path indices for each non-included UTXO.
  - `niLeafLowerRangeValue[niNumberOfUTXOs]` : private signal input for leaf lower range value for each non-included UTXO.
  - `niLeafHigherRangeValue[niNumberOfUTXOs]` : private signal input for leaf higher range value for each non-included UTXO.
  - `niLeafIndex[niNumberOfUTXOs]` : private signal input for the leaf index for each non-included UTXO.

It includes the `MerkleProof` and `NonInclusionProof` components for each UTXO and non-included UTXO,
and sets the signal inputs for each component.
*/

template CombinedMerkleProof(levels, numberOfUTXOs, niNumberOfUTXOs) {

    // public, inclusion
    signal input root[numberOfUTXOs];
    signal input leaf[numberOfUTXOs];
    // private, inclusion
    signal input pathElements[numberOfUTXOs][levels];
    signal input pathIndices[numberOfUTXOs];

    // public, non-inclusion
    signal input niRoot[niNumberOfUTXOs];
    signal input niValue[niNumberOfUTXOs];
    // private, non-inclusion
    signal input niPathElements[niNumberOfUTXOs][levels];
    signal input niPathIndices[niNumberOfUTXOs];
    signal input niLeafLowerRangeValue[niNumberOfUTXOs];
    signal input niLeafHigherRangeValue[niNumberOfUTXOs];
    signal input niLeafIndex[niNumberOfUTXOs];

    component inclusionProofs[numberOfUTXOs];
    for (var i = 0; i < numberOfUTXOs; i++) {
        inclusionProofs[i] = MerkleProof(levels);
        inclusionProofs[i].leaf <== leaf[i];
        inclusionProofs[i].pathIndices <== pathIndices[i];
        inclusionProofs[i].pathElements <== pathElements[i];
        inclusionProofs[i].root === root[i];
    }

    component nonInclusionProofs[niNumberOfUTXOs];
        for (var i = 0; i < niNumberOfUTXOs; i++) {
             nonInclusionProofs[i] = NonInclusionProof(levels);
             nonInclusionProofs[i].root <== niRoot[i];
             nonInclusionProofs[i].value <== niValue[i];

             nonInclusionProofs[i].leafLowerRangeValue <== niLeafLowerRangeValue[i];
             nonInclusionProofs[i].leafHigherRangeValue <== niLeafHigherRangeValue[i];
             nonInclusionProofs[i].leafIndex <== niLeafIndex[i];

             nonInclusionProofs[i].merkleProofHashedIndexedElementLeaf <== niPathElements[i];
             nonInclusionProofs[i].indexHashedIndexedElementLeaf <== niPathIndices[i];
        }
}

