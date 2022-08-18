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
  signal input indices[nIns][nAssets][nAssets];
  signal input amounts[nIns][nAssets];
  signal output out;

  for (var i = 0; i < nIns; i++) {
      for (var j = 0; j < nAssets; j++) {
          var varSumIndices = 0;
          for (var z = 0; z < nAssets; z++) {
              varSumIndices += indices[i][j][z];
              // all indices are 0 or 1
              indices[i][j][z] * (1 - indices[i][j][z]) === 0;
          }
          log(varSumIndices);
          // only one index for one asset is 1
          varSumIndices * (1 - varSumIndices)=== 0;
          // if amount != 0 there should be one an asset assigned to it
          varSumIndices * amounts[i][j] === amounts[i][j];
      }
  }
}


// Universal multi asset JoinSplit transaction with
// nIns s
// nOuts outputs
// nAssets
// one feeAsset at indexOfFeeAsset in assetPubkeys[nAssets]
// the asset in position 1 can be withdrawn
// all other assets can only be used in internal txs
template TransactionAccount(levels, nIns, nOuts, zeroLeaf,indexOfFeeAsset, feeAsset, nAssets) {
    signal input root;
    // extAmount = external amount used for deposits and withdrawals
    // correct extAmount range is enforced on the smart contract
    // publicAmount = extAmount - fee
    signal input publicAmount;
    signal input extDataHash;
    signal input feeAmount;
    signal input mintPubkey;

    signal input  inputNullifier[nIns];
    signal input  inAmount[nIns][nAssets];
    // signal input  inFeeAmount[nIns];
    signal input  inPrivateKey[nIns];
    signal input  inBlinding[nIns];
    signal input  inInstructionType[nIns];

    signal  input inPathIndices[nIns];
    signal  input inPathElements[nIns][levels];

    signal  input inIndices[nIns][nAssets][nAssets];

    // data for transaction outputsAccount
    signal  input outputCommitment[nOuts];
    signal  input outAmount[nOuts][nAssets];
    // signal  input outFeeAmount[nOuts];
    signal  input outPubkey[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outInstructionType[nOuts];
    signal  input outIndices[nOuts][nAssets][nAssets];

    signal  input assetPubkeys[nAssets];

    // feeAsset is asset 0
    assetPubkeys[indexOfFeeAsset] === feeAsset * 1;

    // nAssets should be even plus one feeAsset
    (nAssets - 1) % 2 === 0;

    // defines which utxos are used in which instruction

    component inKeypair[nIns];
    component wrapper[nIns][nAssets][(nAssets - 1) / 2];

    component inCommitmentHasher[nIns];
    component inAmountsHasher[nIns];
    component inAssetsHasher[nIns];

    component inSignature[nIns];
    component inputNullifierHasher[nIns];
    component inTree[nIns];
    component inCheckRoot[nIns];
    component sumIn[nIns][nAssets][nAssets];

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
    for (var i = 0; i < nIns; i++) {
        for (var j = 0; j < nAssets; j++) {
            checkInIndices.amounts[i][j] <== inAmount[i][j];
            for(var z = 0; z < nAssets ; z++) {
                checkInIndices.indices[i][j][z] <== inIndices[i][j][z];
            }
        }

    }

    // verify correctness of transaction s
    for (var tx = 0; tx < nIns; tx++) {

        inKeypair[tx] = Keypair();
        inKeypair[tx].privateKey <== inPrivateKey[tx];

        // determine the asset type
        // and checks that the asset is included in assetPubkeys[nAssets]
        // skips first asset since that is the feeAsset
        // iterates over remaining assets and adds the assetPubkey if index is 1
        // all other indices are zero
        inAssetsHasher[tx] = Poseidon(nAssets);
        for (var a = 0; a < nAssets; a++) {
            var assetPubkey = 0;

            for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
                wrapper[tx][a][i-1] = ADD();
                wrapper[tx][a][i-1].a <== assetPubkeys[i] * inIndices[tx][a][i];
                wrapper[tx][a][i-1].b <== assetPubkeys[i+1] * inIndices[tx][a][i+1];
                assetPubkey += wrapper[tx][a][i-1].out;
            }
            inAssetsHasher[tx].inputs[a] <== assetPubkey  +  (feeAsset * inIndices[tx][a][indexOfFeeAsset]);
        }

        inAmountsHasher[tx] = Poseidon(nAssets);
        for (var a = 0; a < nAssets; a++) {
            inAmountsHasher[tx].inputs[a] <== inAmount[tx][a];
        }
        inCommitmentHasher[tx] = Poseidon(5);
        inCommitmentHasher[tx].inputs[0] <== inAmountsHasher[tx].out;
        inCommitmentHasher[tx].inputs[1] <== inKeypair[tx].publicKey;
        inCommitmentHasher[tx].inputs[2] <== inBlinding[tx];
        inCommitmentHasher[tx].inputs[3] <== inAssetsHasher[tx].out;
        inCommitmentHasher[tx].inputs[4] <== inInstructionType[tx];
        log(inAmount[tx][0]);
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
        // assets should be 0 for zero utxos
        inCheckRoot[tx].enabled <== inCommitmentHasher[tx].inputs[4];

        // sumIns[0] += inFeeAmount[tx];

        for (var i = 0; i < nAssets; i++) {
            for (var j = 0; j < nAssets; j++) {
            log(999999999999);
            log(tx);
            log(i);
            log(j);
            log(inAmount[tx][i]);
            log(inIndices[tx][i][j]);
            log(8888888888);

                sumIn[tx][i][j] = AND();
                sumIn[tx][i][j].a <== inAmount[tx][i];
                sumIn[tx][i][j].b <== inIndices[tx][i][j];
                log(sumIn[tx][i][j].out);
                sumIns[j] += sumIn[tx][i][j].out;
            }
            log(i);
            log(sumIns[2]);
            log(9999999999991);

            // sumIns[a] += inAmount[tx][a];
        }

        // check asset type for withdrawal
        // asset has to be in
        selectorCheckMint[tx] = AND();
        selectorCheckMint[tx].a <== mintPubkey;
        selectorCheckMint[tx].b <== inIndices[1][1][tx];

        inCheckMint[tx] = ForceEqualIfEnabled();
        inCheckMint[tx].in[0] <== inCommitmentHasher[tx].inputs[3];
        inCheckMint[tx].in[1] <== mintPubkey;
        inCheckMint[tx].enabled <== selectorCheckMint[tx].out;

    }

    component outWrapper[nOuts][nAssets][(nAssets - 1) / 2];
    component outCommitmentHasher[nOuts];
    component outAmountCheck[nOuts][nAssets];
    component sumOut[nIns][nAssets][nAssets];
    component outAmountHasher[nOuts];
    component outAssetHasher[nOuts];

    var sumOuts[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumOuts[i] = 0;
    }

    component checkOutIndices = CheckIndices(nOuts, nAssets);
    for (var i = 0; i < nOuts; i++) {
        for (var j = 0; j < nAssets; j++) {
          checkOutIndices.amounts[i][j] <== outAmount[i][j];

          for(var z = 0; z < nAssets; z++) {
            checkOutIndices.indices[i][j][z] <== outIndices[i][j][z];
          }
        }
    }

    // verify correctness of transaction outputs
    for (var tx = 0; tx < nOuts; tx++) {


        // for every asset for every tx only one index is 1 others are 0
        // select the asset corresponding to the index
        // and add it to the assetHasher
        outAssetHasher[tx] = Poseidon(nAssets);
        for (var a = 0; a < nAssets; a++) {
            var assetPubkey = 0;
            for (var i = 1; i <= (nAssets - 1) / 2; i+=2) {
            outWrapper[tx][a][i-1] = ADD();
            outWrapper[tx][a][i-1].a <== assetPubkeys[i] * outIndices[tx][a][i];
            outWrapper[tx][a][i-1].b <== assetPubkeys[i+1] * outIndices[tx][a][i+1];
            assetPubkey += outWrapper[tx][a][i-1].out;
            }
            outAssetHasher[tx].inputs[a] <== assetPubkey + feeAsset * outIndices[tx][a][indexOfFeeAsset];
        }

        for (var i = 0; i < nAssets; i++) {
            // Check that amount fits into 248 bits to prevent overflow
            outAmountCheck[tx][i] = Num2Bits(248);
            outAmountCheck[tx][i].in <== outAmount[tx][i];
        }
        log(77777777);
        outAmountHasher[tx] = Poseidon(nAssets);
        for (var i = 0; i < nAssets; i++) {
            log(outAmount[tx][i]);
            outAmountHasher[tx].inputs[i] <== outAmount[tx][i];
        }
        log(77777777);
        outCommitmentHasher[tx] = Poseidon(5);
        outCommitmentHasher[tx].inputs[0] <== outAmountHasher[tx].out;
        outCommitmentHasher[tx].inputs[1] <== outPubkey[tx];
        outCommitmentHasher[tx].inputs[2] <== outBlinding[tx];
        outCommitmentHasher[tx].inputs[3] <== outAssetHasher[tx].out;
        outCommitmentHasher[tx].inputs[4] <== outInstructionType[tx];
        log(outCommitmentHasher[tx].inputs[0]);
        log(outCommitmentHasher[tx].inputs[1]);
        log(outCommitmentHasher[tx].inputs[2]);
        log(outCommitmentHasher[tx].inputs[3]);
        log(outCommitmentHasher[tx].inputs[4]);
        log(outCommitmentHasher[tx].out);
        log(outCommitmentHasher[tx].out == outputCommitment[tx]);
        log(outputCommitment[tx]);
        log(5);

        outCommitmentHasher[tx].out === outputCommitment[tx];

        // sumOuts[0] += outFeeAmount[tx];

        // Increases sumOuts of the correct asset by outAmount
        for (var i = 0; i < nAssets; i++) {
            for (var j = 0; j < nAssets; j++) {
                sumOut[tx][i][j] = AND();
                sumOut[tx][i][j].a <== outAmount[tx][i];
                sumOut[tx][i][j].b <== outIndices[tx][i][j];
                sumOuts[j] += sumOut[tx][i][j].out;
            }
            // sumIns[a] += inAmount[tx][a];
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
