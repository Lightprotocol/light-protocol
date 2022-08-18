include "../../node_modules/circomlib/circuits/poseidon.circom";
include "./merkleProof.circom"
include "./keypair.circom"
include "../../node_modules/circomlib/circuits/gates.circom"
// include "../../node_modules/circomlib/circuits/bitify.circom"


/*
Utxo structure:
{
    amount,
    pubkey,
    blinding, // random number
}

commitment = hash(amount, pubKey, blinding)
nullifier = hash(commitment, merklePath, sign(privKey, commitment, merklePath))
*/

template ADD() {
    signal input a;
    signal input b;
    signal output out;

    out <== a + b;
}


template Wrapper() {
    signal input addA;
    signal input addB;
    signal input feePayingAssetIndex;
    signal output out;
    component add = ADD();
    add.a <== addA;
    add.b <== addB;
    component not = NOT();
    not.in <== feePayingAssetIndex;
    out <== not.out * add.out;
}

// doesn't do anything
template CheckIndices(nIns, nAssets) {
  signal input indices[nAssets][nIns];
  signal output out;

  for (var i = 0; i < nIns; i++) {
    var varSumIndices = 0;
      for (var j = 0; j < nAssets; j++) {
        varSumIndices += indices[j][i];
          for (var a = j + 1; a < nAssets; a++) {
            0 === indices[j][i] * indices[a][i];
        }
      }
      1 === varSumIndices;
  }
  out <== 1;
}


// Universal multi asset JoinSplit transaction with
// nIns inputs
// nOuts outputs
// nAssets
// one feeAsset at indexOfFeeAsset in assetPubkeys[nAssets]
// the asset in position 1 can be withdrawn
// all other assets can only be used in internal txs
template Transaction(levels, nIns, nOuts, zeroLeaf,indexOfFeeAsset, feeAsset, nAssets) {
    signal input root;
    // extAmount = external amount used for deposits and withdrawals
    // correct extAmount range is enforced on the smart contract
    // publicAmount = extAmount - fee
    signal input publicAmount;
    signal input extDataHash;
    signal input feeAmount;
    signal input mintPubkey;

    // data for transaction inputs
    signal         input inputNullifier[nIns];
    signal private input inAmount[nIns];
    signal private input inPrivateKey[nIns];
    signal private input inBlinding[nIns];
    signal private input inInstructionType[nIns];

    signal private input inPathIndices[nIns];
    signal private input inPathElements[nIns][levels];
    signal private input inIndices[nAssets][nIns];

    // data for transaction outputs
    signal         input outputCommitment[nOuts];
    signal private input outAmount[nOuts];
    signal private input outPubkey[nOuts];
    signal private input outBlinding[nOuts];
    signal private input outInstructionType[nOuts];
    signal private input outIndices[nAssets][nOuts];
    // swap constraints
    signal private input constraint[3];

    // feeAsset is asset 0
    // normal asset is asset 1
    // swap asset is asset 3
    signal private input assetPubkeys[nAssets];
    assetPubkeys[indexOfFeeAsset] === feeAsset * 1;

    // defines which utxos are used in which instruction
    // signal private input instructionIndices[nIns][nIns];
    // defines the private constraint values to be passed into
    // signal private input instructionConstraintValues[nIns][nConstraints];


    component inKeypair[nIns];
    component wrapper[nIns][(nAssets - 1) / 2];

    component inCommitmentHasher[nIns];
    component inSignature[nIns];
    component inNullifierHasher[nIns];
    component inTree[nIns];
    component inCheckRoot[nIns];
    component sumIn[nIns][nAssets];

    component inCheckMint[nIns];
    component selectorCheckMint[nIns];

    // swap components
    component checkSwapConstraint[nIns];
    component inCheckconstraintInBlinding[nIns];
    component inCheckconstraintAmount[nIns];
    component inCheckconstraintPubkey[nIns];
    component inCheckconstraintAsset[nIns];
    component checkSwapConstraint1[nIns];
    component inCheckconstraintOutBlinding[nIns];
    component inCheckconstraintOutInstructionType[nIns];

    var sumIns[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumIns[i] = 0;
    }

    component checkInIndices = CheckIndices(nIns, nAssets);
    for (var a = 0; a < nAssets; a++) {
        for(var i = 0; i < nIns; i++) {
            checkInIndices.indices[a][i] <== inIndices[a][i];
        }
    }
    component getInstructions[nIns]


    // verify correctness of transaction inputs
    for (var tx = 0; tx < nIns; tx++) {
        getInstructions[tx] = Num2Bits(248);

        getInstructions[tx].in <== inInstructionType[tx];

        var instructions[32];
        /*
        for (var i = 0; i < 248; i++) {
            instructions[i] = getInstructions.out[i];
        }*/


        inKeypair[tx] = Keypair();
        inKeypair[tx].privateKey <== inPrivateKey[tx];
        var assetId = 0;
        for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
            wrapper[tx][i-1] = Wrapper();
            wrapper[tx][i-1].addA <== assetPubkeys[i] * inIndices[i][tx];
            wrapper[tx][i-1].addB <== assetPubkeys[i+1] * inIndices[i+1][tx];
            wrapper[tx][i-1].feePayingAssetIndex <== inIndices[indexOfFeeAsset][tx]
            assetId += wrapper[tx][i-1].out;
        }

        inCommitmentHasher[tx] = Poseidon(5);
        inCommitmentHasher[tx].inputs[0] <== inAmount[tx];
        inCommitmentHasher[tx].inputs[1] <== inKeypair[tx].publicKey;
        inCommitmentHasher[tx].inputs[2] <== inBlinding[tx];
        inCommitmentHasher[tx].inputs[3] <== assetId  +  (feeAsset * inIndices[indexOfFeeAsset][tx]);
        inCommitmentHasher[tx].inputs[4] <== inInstructionType[tx];


        inSignature[tx] = Signature();
        inSignature[tx].privateKey <== inPrivateKey[tx];
        inSignature[tx].commitment <== inCommitmentHasher[tx].out;
        inSignature[tx].merklePath <== inPathIndices[tx];

        inNullifierHasher[tx] = Poseidon(3);
        inNullifierHasher[tx].inputs[0] <== inCommitmentHasher[tx].out;
        inNullifierHasher[tx].inputs[1] <== inPathIndices[tx];
        inNullifierHasher[tx].inputs[2] <== inSignature[tx].out;
        inNullifierHasher[tx].out === inputNullifier[tx];

        inTree[tx] = MerkleProof(levels);
        inTree[tx].leaf <== inCommitmentHasher[tx].out;
        inTree[tx].pathIndices <== inPathIndices[tx];
        for (var i = 0; i < levels; i++) {
            inTree[tx].pathElements[i] <== inPathElements[tx][i];
        }

        // check merkle proof only if amount is non-zero
        inCheckRoot[tx] = ForceEqualIfEnabled();
        inCheckRoot[tx].in[0] <== root;
        inCheckRoot[tx].in[1] <== inTree[tx].root;
        inCheckRoot[tx].enabled <== inAmount[tx];
        for (var a = 0; a < nAssets; a++) {

            sumIn[tx][a] = AND();
            sumIn[tx][a].a <== inAmount[tx];
            sumIn[tx][a].b <== inIndices[a][tx];
            sumIns[a] += sumIn[tx][a].out;

        }

        // check asset type for withdrawal
        // asset has to be in
        selectorCheckMint[tx] = AND();
        selectorCheckMint[tx].a <== mintPubkey;
        selectorCheckMint[tx].b <== inIndices[1][tx];

        inCheckMint[tx] = ForceEqualIfEnabled();
        inCheckMint[tx].in[0] <== inCommitmentHasher[tx].inputs[3];
        inCheckMint[tx].in[1] <== mintPubkey;
        inCheckMint[tx].enabled <== selectorCheckMint[tx].out;

        // Instructions
        // what if I aggregate the results into one constraint
        // and multiply it with 0
        // maybe this way I can reduce prooftime
        checkSwapConstraint[tx] = Poseidon(6);
        checkSwapConstraint[tx].inputs[0] <== constraint[0];
        checkSwapConstraint[tx].inputs[1] <== constraint[1];
        checkSwapConstraint[tx].inputs[2] <== constraint[2];
        checkSwapConstraint[tx].inputs[0] <== constraint[3];
        checkSwapConstraint[tx].inputs[1] <== constraint[4];
        checkSwapConstraint[tx].inputs[2] <== constraint[5];

        inCheckconstraintInBlinding[tx] = ForceEqualIfEnabled();
        inCheckconstraintInBlinding[tx].in[0] <== checkSwapConstraint[tx].out;
        inCheckconstraintInBlinding[tx].in[1] <== inBlinding[tx];
        inCheckconstraintInBlinding[tx].enabled <== inInstructionType[tx];

        // if (tx == 2) {
            // if is instruction enforce

            // swap constraints:
            // check amount
            // check destination pubkey
            // check destination asset
            // for(var n = 0; n < nConstraints; n++) {

            // tokens to swap can be deposited in the same tx
            inCheckconstraintAmount[tx] = ForceEqualIfEnabled();
            inCheckconstraintAmount[tx].in[0] <== constraint[0];
            inCheckconstraintAmount[tx].in[1] <== outAmount[1];
            inCheckconstraintAmount[tx].enabled <== inInstructionType[tx];

            inCheckconstraintPubkey[tx] = ForceEqualIfEnabled();
            inCheckconstraintPubkey[tx].in[0] <== constraint[1];
            inCheckconstraintPubkey[tx].in[1] <== outPubkey[1];
            inCheckconstraintPubkey[tx].enabled <== inInstructionType[tx];

            // write custom ForceEqualIfEnabled(N) enabled with N
            inCheckconstraintAsset[tx] = ForceEqualIfEnabled();
            inCheckconstraintAsset[tx].in[0] <== constraint[2];
            inCheckconstraintAsset[tx].in[1] <== assetPubkeys[1] * outIndices[1][tx];
            inCheckconstraintAsset[tx].enabled <== inInstructionType[tx];

            checkSwapConstraint1[tx] = Poseidon(2);
            checkSwapConstraint1[tx].inputs[0] <== inBlinding[tx];
            checkSwapConstraint1[tx].inputs[1] <== inBlinding[tx];
            // checkSwapConstraint1[tx].inputs[2] <== inBlinding[tx];

            inCheckconstraintOutBlinding[tx] = ForceEqualIfEnabled();
            inCheckconstraintOutBlinding[tx].in[0] <== checkSwapConstraint1[tx].out;
            inCheckconstraintOutBlinding[tx].in[1] <== outBlinding[1];
            inCheckconstraintOutBlinding[tx].enabled <== inInstructionType[tx];

            inCheckconstraintOutInstructionType[tx] = ForceEqualIfEnabled();
            inCheckconstraintOutInstructionType[tx].in[0] <== 0;
            inCheckconstraintOutInstructionType[tx].in[1] <== outInstructionType[1];
            inCheckconstraintOutInstructionType[tx].enabled <== inInstructionType[tx];



        // }
    }



    component outWrapper[nOuts][(nAssets - 1) / 2];
    component outCommitmentHasher[nOuts];
    component outAmountCheck[nOuts];
    component sumOut[nIns][nAssets];

    var sumOuts[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumOuts[i] = 0;
    }
    component checkOutIndices = CheckIndices(nOuts, nAssets);
    for (var a = 0; a < nAssets; a++) {
        for(var i = 0; i < nOuts; i++) {
            checkOutIndices.indices[a][i] <== outIndices[a][i];
        }
    }

    // verify correctness of transaction outputs
    for (var tx = 0; tx < nOuts; tx++) {

        var assetId = 0;
        for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
            outWrapper[tx][i-1] = Wrapper();
            outWrapper[tx][i-1].addA <== assetPubkeys[i] * outIndices[i][tx];
            outWrapper[tx][i-1].addB <== assetPubkeys[i+1] * outIndices[i+1][tx];
            outWrapper[tx][i-1].feePayingAssetIndex <== outIndices[indexOfFeeAsset][tx]
            assetId += outWrapper[tx][i-1].out;
        }


        // Check that amount fits into 248 bits to prevent overflow
        outAmountCheck[tx] = Num2Bits(248);
        outAmountCheck[tx].in <== outAmount[tx];

        outCommitmentHasher[tx] = Poseidon(5);
        outCommitmentHasher[tx].inputs[0] <== outAmount[tx];
        outCommitmentHasher[tx].inputs[1] <== outPubkey[tx];
        outCommitmentHasher[tx].inputs[2] <== outBlinding[tx];
        outCommitmentHasher[tx].inputs[3] <== assetId + feeAsset * outIndices[indexOfFeeAsset][tx];
        outCommitmentHasher[tx].inputs[4] <== outInstructionType[tx];
        outCommitmentHasher[tx].out === outputCommitment[tx];

        for (var a = 0; a < nAssets; a++) {
            sumOut[tx][a] = AND();
            sumOut[tx][a].a <== outAmount[tx];
            sumOut[tx][a].b <== outIndices[a][tx];
            sumOuts[a] += sumOut[tx][a].out;
        }

    }

    // check that there are no same nullifiers among all inputs
    component sameNullifiers[nIns * (nIns - 1) / 2];
    var index = 0;
    for (var i = 0; i < nIns - 1; i++) {
      for (var j = i + 1; j < nIns; j++) {
          sameNullifiers[index] = IsEqual();
          sameNullifiers[index].in[0] <== inputNullifier[i];
          sameNullifiers[index].in[1] <== inputNullifier[j];
          sameNullifiers[index].out === 0;
          index++;
      }
    }

    // verify amount invariant
    sumIns[0] + feeAmount === sumOuts[0];
    sumIns[1] + publicAmount === sumOuts[1];

    for (var a = 2; a < nAssets; a++) {
      sumIns[a] === sumOuts[a];
    }


    // optional safety constraint to make sure extDataHash cannot be changed
    signal extDataSquare <== extDataHash * extDataHash;


}
