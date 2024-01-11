pragma circom 2.1.4;
include "poseidon.circom";
include "gates.circom";

template CheckProgramTransaction(nIns, nOuts) {
    signal input publicProgramId;
    signal input inVerifierPublicKey[nIns];
    signal input inUtxoHash[nIns];
    signal input outUtxoHash[nOuts];
    signal input publicDataHash;
    signal input publicTransactionHash;

    signal output transactionHash;

    component inVerifierCheck[nIns];
    for (var tx = 0; tx < nIns; tx++) {
        // if inDataHash is not 0 check publicProgramIdPublicKey === inVerifierPublicKey[tx]
        inVerifierCheck[tx] = ForceEqualIfEnabled();
        inVerifierCheck[tx].in[0] <== publicProgramId;
        inVerifierCheck[tx].in[1] <== inVerifierPublicKey[tx];
        inVerifierCheck[tx].enabled <== inVerifierPublicKey[tx]; // switch to utxoDataHash
    }

    component transactionHasher =  ComputeTransactionHash(nIns, nOuts);
    transactionHasher.inUtxoHash <== inUtxoHash;
    transactionHasher.outUtxoHash <== outUtxoHash;
    transactionHasher.publicDataHash <== publicDataHash;

    publicTransactionHash === transactionHasher.transactionHash;
    transactionHash <== transactionHasher.transactionHash;
}

template ComputeTransactionHash(nIns, nOuts) {
    signal input inUtxoHash[nIns];
    signal input outUtxoHash[nOuts];
    signal input publicDataHash;

    signal output transactionHash;

    
    // hash commitment 
    component inputHasher = Poseidon(nIns);
    for (var i = 0; i < nIns; i++) {
        inputHasher.inputs[i] <== inUtxoHash[i];
    }

    component outputHasher = Poseidon(nOuts);
    for (var i = 0; i < nOuts; i++) {
        outputHasher.inputs[i] <== outUtxoHash[i];
    }

    component transactionHasher = Poseidon(3);

    transactionHasher.inputs[0] <== inputHasher.out;
    transactionHasher.inputs[1] <== outputHasher.out;
    transactionHasher.inputs[2] <== publicDataHash;
    transactionHash <== transactionHasher.out;
}