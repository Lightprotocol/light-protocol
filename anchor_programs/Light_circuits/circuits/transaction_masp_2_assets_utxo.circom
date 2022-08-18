pragma circom 2.0.0;
include "../../node_modules/circomlib/circuits/poseidon.circom";
include "./merkleProof.circom";
include "./keypair.circom";
include "../../node_modules/circomlib/circuits/gates.circom";


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
// helper function to adhere to quadratic constraints
template ADD() {
    signal input a;
    signal input b;
    signal output out;

    out <== a + b;
}


// Checks that that for every i there is only one index == 1 for all assets
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
}


// Universal multi asset JoinSplit transaction with
// nIns s
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

    // data for transaction s
    signal input  inputNullifier[nIns];
    signal input  inAmount[nIns];
    signal input  inFeeAmount[nIns];
    signal input  inPrivateKey[nIns];
    signal input  inBlinding[nIns];
    signal input  inInstructionType[nIns];

    signal  input inPathIndices[nIns];
    signal  input inPathElements[nIns][levels];

    signal  input inIndices[nAssets][nIns];

    // data for transaction outputs
    signal  input outputCommitment[nOuts];
    signal  input outAmount[nOuts];
    signal  input outFeeAmount[nOuts];
    signal  input outPubkey[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outInstructionType[nOuts];
    signal  input outIndices[nAssets][nOuts];

    signal  input assetPubkeys[nAssets];

    // feeAsset is asset 0
    assetPubkeys[indexOfFeeAsset] === feeAsset * 1;

    // nAssets should be even plus one feeAsset
    (nAssets - 1) % 2 === 0;

    // defines which utxos are used in which instruction

    component inKeypair[nIns];
    component wrapper[nIns][(nAssets - 1) / 2];

    component inCommitmentHasher[nIns];
    component inSignature[nIns];
    component inputNullifierHasher[nIns];
    component inTree[nIns];
    component inCheckRoot[nIns];
    component sumIn[nIns][nAssets];

    component inCheckMint[nIns];
    component selectorCheckMint[nIns];

    var sumIns[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumIns[i] = 0;
    }
    log(1);
    // checks that all indices are either 0 or 1
    // checks that for every utxo exactly one asset is defined

    component checkInIndices;
    checkInIndices = CheckIndices(nIns, nAssets);
    for (var a = 0; a < nAssets; a++) {
        for(var i = 0; i < nIns; i++) {
            checkInIndices.indices[a][i] <== inIndices[a][i];
        }
    }
    log(1);


    // verify correctness of transaction s
    for (var tx = 0; tx < nIns; tx++) {

        inKeypair[tx] = Keypair();
        inKeypair[tx].privateKey <== inPrivateKey[tx];

        // determine the asset type
        // and checks that the asset is included in assetPubkeys[nAssets]
        var assetPubkey = 0;
        // skips first asset since that is the feeAsset
        // iterates over remaining assets and adds the assetPubkey if index is 1
        // all other indices are zero
        for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
            wrapper[tx][i-1] = ADD();
            wrapper[tx][i-1].a <== assetPubkeys[i] * inIndices[i][tx];
            wrapper[tx][i-1].b <== assetPubkeys[i+1] * inIndices[i+1][tx];
            assetPubkey += wrapper[tx][i-1].out;
        }

        inCommitmentHasher[tx] = Poseidon(6);
        inCommitmentHasher[tx].inputs[0] <== inAmount[tx];
        inCommitmentHasher[tx].inputs[1] <== inFeeAmount[tx];
        inCommitmentHasher[tx].inputs[2] <== inKeypair[tx].publicKey;
        inCommitmentHasher[tx].inputs[3] <== inBlinding[tx];
        inCommitmentHasher[tx].inputs[4] <== assetPubkey  +  (feeAsset * inIndices[indexOfFeeAsset][tx]);
        inCommitmentHasher[tx].inputs[5] <== inInstructionType[tx];
        log(inAmount[tx]);
        log(inKeypair[tx].publicKey);
        log(inBlinding[tx]);
        log(inCommitmentHasher[tx].inputs[3]);
        log(inCommitmentHasher[tx].inputs[4]);
        log(inCommitmentHasher[tx].out);
        log(11111);

        inSignature[tx] = Signature();
        inSignature[tx].privateKey <== inPrivateKey[tx];
        inSignature[tx].commitment <== inCommitmentHasher[tx].out;
        inSignature[tx].merklePath <== inPathIndices[tx];

        inputNullifierHasher[tx] = Poseidon(3);
        inputNullifierHasher[tx].inputs[0] <== inCommitmentHasher[tx].out;
        inputNullifierHasher[tx].inputs[1] <== inPathIndices[tx];
        inputNullifierHasher[tx].inputs[2] <== inSignature[tx].out;
        log(inCommitmentHasher[tx].out);
        log(inPathIndices[tx]);
        log(inSignature[tx].out);
        log(inputNullifierHasher[tx].out);
        log(inputNullifierHasher[tx].out == inputNullifier[tx]);
        log(inputNullifier[tx]);
        log(222222);


        inputNullifierHasher[tx].out === inputNullifier[tx];

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

        sumIns[0] += inFeeAmount[tx];

        for (var a = 1; a < nAssets; a++) {

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


        // for every asset for every tx only one index is 1 others are 0
        // select the asset corresponding to the index
        var assetPubkey = 0;
        for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
            outWrapper[tx][i-1] = ADD();
            outWrapper[tx][i-1].a <== assetPubkeys[i] * outIndices[i][tx];
            outWrapper[tx][i-1].b <== assetPubkeys[i+1] * outIndices[i+1][tx];
            assetPubkey += outWrapper[tx][i-1].out;
        }


        // Check that amount fits into 248 bits to prevent overflow
        outAmountCheck[tx] = Num2Bits(248);
        outAmountCheck[tx].in <== outAmount[tx];

        outCommitmentHasher[tx] = Poseidon(6);
        outCommitmentHasher[tx].inputs[0] <== outAmount[tx];
        outCommitmentHasher[tx].inputs[1] <== outFeeAmount[tx];
        outCommitmentHasher[tx].inputs[2] <== outPubkey[tx];
        outCommitmentHasher[tx].inputs[3] <== outBlinding[tx];
        outCommitmentHasher[tx].inputs[4] <== assetPubkey + feeAsset * outIndices[indexOfFeeAsset][tx];
        outCommitmentHasher[tx].inputs[5] <== outInstructionType[tx];

        outCommitmentHasher[tx].out === outputCommitment[tx];
        log(5);
        log(outCommitmentHasher[tx].out == outputCommitment[tx]);

        sumOuts[0] += outFeeAmount[tx];

        // Increases sumOuts of the correct asset by outAmount
        for (var a = 1; a < nAssets; a++) {
            sumOut[tx][a] = AND();
            sumOut[tx][a].a <== outAmount[tx];
            sumOut[tx][a].b <== outIndices[a][tx];
            sumOuts[a] += sumOut[tx][a].out;
        }

    }

    // check that there are no same nullifiers among all s
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

    log(11111111111111111111111111111112);
    log(sumIns[0]);
    log(feeAmount);
    log(sumOuts[0]);
    log(11111111111111111111111111111111);
    log(sumIns[1]);
    log(publicAmount);
    log(sumOuts[1]);


    // verify amount invariant
    sumIns[0] + feeAmount === sumOuts[0];
    sumIns[1] + publicAmount === sumOuts[1];

    for (var a = 2; a < nAssets; a++) {
      log(11111111111111111111111111111112);
      log(sumIns[a]);
      log(sumOuts[a]);
      sumIns[a] === sumOuts[a];
    }


    // optional safety constraint to make sure extDataHash cannot be changed
    signal extDataSquare <== extDataHash * extDataHash;


}
