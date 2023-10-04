pragma circom 2.0.0;
include "poseidon.circom";
include "../merkle-tree/merkleProof.circom";
include "../light-utils/keypair.circom";
include "gates.circom";


/*
Utxo structure:
{
    amount,
    pubkey,
    blinding, // random number
}

commitment = hash(amountHash, pubKey, blinding, assetHash, appDataHash)
nullifier = hash(commitment, merklePath, sign(privKey, commitment, merklePath))
*/

// Checks that that for every i there is only one index == 1 for all assets
template CheckIndices(n, nInAssets, nAssets) {
  signal input indices[n][nInAssets][nAssets];
  signal input amounts[n][nInAssets];

  for (var i = 0; i < n; i++) {
      for (var j = 0; j < nInAssets; j++) {
          var varSumIndices = 0;
          for (var z = 0; z < nAssets; z++) {
              varSumIndices += indices[i][j][z];
              // all indices are 0 or 1
              indices[i][j][z] * (1 - indices[i][j][z]) === 0;
          }
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
// one feeAsset at indexFeeAsset in assetPubkeys[nAssets]
// the asset in position 1 can be unshielded
// all other assets can only be used in internal txs
template TransactionAccount(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets) {

    // Range Check to prevent an overflow of wrong circuit instantiation
    assert( nIns * nAssets < 1000);
    assert( nInAssets <= nAssets);
    assert( nOutAssets <= nAssets);

    signal input root;
    // extAmount = external amount used for shields and unshields
    // correct extAmount range is enforced on the smart contract
    // publicAmountSpl = extAmount - fee
    signal input publicAmountSpl;
    signal input txIntegrityHash;
    signal input publicAmountSol;
    signal input publicMintPubkey;

    signal input  inputNullifier[nIns];
    signal input  inAmount[nIns][nInAssets];
    signal input  inPrivateKey[nIns];
    signal input  inBlinding[nIns];
    signal input  inAppDataHash[nIns];

    signal  input inPathIndices[nIns];
    signal  input inPathElements[nIns][levels];

    signal  input inIndices[nIns][nInAssets][nAssets];

    // data for transaction outputsAccount
    signal  input outputCommitment[nOuts];
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outPubkey[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outAppDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];

    
    signal  input outPoolType[nOuts];
    signal  input outVerifierPubkey[nOuts];

    signal  input inPoolType[nIns];
    signal  input inVerifierPubkey[nIns];
    signal input transactionVersion;

    // enforce pooltypes of 0
    // add public input to distinguish between pool types
    inPoolType[0] === 0;
    inPoolType[0] === outPoolType[0];
    for (var tx = 0; tx < nIns; tx++) {
        inAppDataHash[tx] === 0;
        inVerifierPubkey[tx] === 0;
    }

    
    signal  input assetPubkeys[nAssets];

    // feeAsset is asset indexFeeAsset
    assetPubkeys[indexFeeAsset] === feeAsset;

    // If public amount is != 0 then check that assetPubkeys[indexPublicAsset] == publicMintPubkey

    component checkMintPubkey = ForceEqualIfEnabled();
    checkMintPubkey.in[0] <== assetPubkeys[indexPublicAsset];
    checkMintPubkey.in[1] <== publicMintPubkey;

    checkMintPubkey.enabled <== publicAmountSpl;

    component assetCheck[nAssets];
    for (var i = 0; i < nAssets; i++) {
        assetCheck[i] = Num2Bits(248);
        assetCheck[i].in <== assetPubkeys[i];
    }

    component inKeypair[nIns];
    component inGetAsset[nIns][nInAssets][nAssets];

    component inCommitmentHasher[nIns];
    component inAmountsHasher[nIns];
    component inAssetsHasher[nIns];

    component inSignature[nIns];
    component inputNullifierHasher[nIns];
    component inTree[nIns];
    component inCheckRoot[nIns];
    component sumIn[nIns][nInAssets][nAssets];
    component inAmountCheck[nIns][nInAssets];

    component inCheckMint[nIns];
    component selectorCheckMint[nIns];
    var sumIns[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumIns[i] = 0;
    }

    // checks that all indices are either 0 or 1
    // checks that there is exactly one asset defined for every utxo
    component checkInIndices;
    checkInIndices = CheckIndices(nIns, nInAssets, nAssets);
    for (var i = 0; i < nIns; i++) {
        for (var j = 0; j < nInAssets; j++) {
            checkInIndices.amounts[i][j] <== inAmount[i][j];
            for(var z = 0; z < nAssets ; z++) {
                checkInIndices.indices[i][j][z] <== inIndices[i][j][z];
            }
        }
    }

    // verify correctness of transaction s
    for (var tx = 0; tx < nIns; tx++) {

        inPoolType[0] === inPoolType[tx];

        inKeypair[tx] = Keypair();
        inKeypair[tx].privateKey <== inPrivateKey[tx];

        // determine the asset type
        // and checks that the asset is included in assetPubkeys[nInAssets]
        // skips first asset since that is the feeAsset
        // iterates over remaining assets and adds the assetPubkey if index is 1
        // all other indices are zero
        inAssetsHasher[tx] = Poseidon(nInAssets);
        for (var a = 0; a < nInAssets; a++) {
            var assetPubkey = 0;

            for (var i = 0; i < nAssets; i++) {
                inGetAsset[tx][a][i] = AND();
                inGetAsset[tx][a][i].a <== assetPubkeys[i];
                inGetAsset[tx][a][i].b <== inIndices[tx][a][i];
                assetPubkey += inGetAsset[tx][a][i].out;
            }
            inAssetsHasher[tx].inputs[a] <== assetPubkey;
        }

        inAmountsHasher[tx] = Poseidon(nInAssets);
        var sumInAmount = 0;
        for (var a = 0; a < nInAssets; a++) {
            inAmountCheck[tx][a] = Num2Bits(64);
            inAmountCheck[tx][a].in <== inAmount[tx][a];
            inAmountsHasher[tx].inputs[a] <== inAmount[tx][a];
            sumInAmount += inAmount[tx][a];
        }

        inCommitmentHasher[tx] = Poseidon(8);
        inCommitmentHasher[tx].inputs[0] <== transactionVersion; // transaction version
        inCommitmentHasher[tx].inputs[1] <== inAmountsHasher[tx].out;
        inCommitmentHasher[tx].inputs[2] <== inKeypair[tx].publicKey;
        inCommitmentHasher[tx].inputs[3] <== inBlinding[tx];
        inCommitmentHasher[tx].inputs[4] <== inAssetsHasher[tx].out;
        inCommitmentHasher[tx].inputs[5] <== inAppDataHash[tx];
        inCommitmentHasher[tx].inputs[6] <== inPoolType[tx];
        inCommitmentHasher[tx].inputs[7] <== inVerifierPubkey[tx];

        inSignature[tx] = Signature();
        inSignature[tx].privateKey <== inPrivateKey[tx];
        inSignature[tx].commitment <== inCommitmentHasher[tx].out;
        inSignature[tx].merklePath <== inPathIndices[tx];

        inputNullifierHasher[tx] = Poseidon(3);
        inputNullifierHasher[tx].inputs[0] <== inCommitmentHasher[tx].out;
        inputNullifierHasher[tx].inputs[1] <== inPathIndices[tx];
        inputNullifierHasher[tx].inputs[2] <== inSignature[tx].out;

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
        inCheckRoot[tx].enabled <== sumInAmount;

        for (var i = 0; i < nInAssets; i++) {
            for (var j = 0; j < nAssets; j++) {
                sumIn[tx][i][j] = AND();
                sumIn[tx][i][j].a <== inAmount[tx][i];
                sumIn[tx][i][j].b <== inIndices[tx][i][j];
                sumIns[j] += sumIn[tx][i][j].out;
            }

        }
    }

    component outGetAsset[nOuts][nOutAssets][nAssets];
    component outCommitmentHasher[nOuts];
    component outAmountCheck[nOuts][nOutAssets];
    component sumOut[nOuts][nOutAssets][nAssets];
    component outAmountsHasher[nOuts];
    component outAssetsHasher[nOuts];

    var sumOuts[nAssets];
    for (var i = 0; i < nAssets; i++) {
      sumOuts[i] = 0;
    }

    component checkOutIndices = CheckIndices(nOuts,nOutAssets, nAssets);
    for (var i = 0; i < nOuts; i++) {
        for (var j = 0; j < nOutAssets; j++) {
          checkOutIndices.amounts[i][j] <== outAmount[i][j];

          for(var z = 0; z < nAssets; z++) {
            checkOutIndices.indices[i][j][z] <== outIndices[i][j][z];
          }
        }
    }

    // verify correctness of transaction outputs
    for (var tx = 0; tx < nOuts; tx++) {

        outPoolType[0] === outPoolType[tx];
        // for every asset for every tx only one index is 1 others are 0
        // select the asset corresponding to the index
        // and add it to the assetHasher
        outAssetsHasher[tx] = Poseidon(nOutAssets);

        for (var a = 0; a < nOutAssets; a++) {
            var assetPubkey = 0;

            for (var i = 0; i < nAssets; i++) {
                outGetAsset[tx][a][i] = AND();
                outGetAsset[tx][a][i].a <== assetPubkeys[i];
                outGetAsset[tx][a][i].b <== outIndices[tx][a][i];
                assetPubkey += outGetAsset[tx][a][i].out;
            }
            outAssetsHasher[tx].inputs[a] <== assetPubkey;
        }

        for (var i = 0; i < nOutAssets; i++) {
            // Check that amount fits into 64 bits to prevent overflow
            outAmountCheck[tx][i] = Num2Bits(64);
            outAmountCheck[tx][i].in <== outAmount[tx][i];
        }

        outAmountsHasher[tx] = Poseidon(nOutAssets);
        for (var i = 0; i < nOutAssets; i++) {
            outAmountsHasher[tx].inputs[i] <== outAmount[tx][i];
        }

        outCommitmentHasher[tx] = Poseidon(8);
        outCommitmentHasher[tx].inputs[0] <== transactionVersion; // transaction version
        outCommitmentHasher[tx].inputs[1] <== outAmountsHasher[tx].out;
        outCommitmentHasher[tx].inputs[2] <== outPubkey[tx];
        outCommitmentHasher[tx].inputs[3] <== outBlinding[tx];
        outCommitmentHasher[tx].inputs[4] <== outAssetsHasher[tx].out;
        outCommitmentHasher[tx].inputs[5] <== outAppDataHash[tx];
        outCommitmentHasher[tx].inputs[6] <== outPoolType[tx];
        outCommitmentHasher[tx].inputs[7] <== outVerifierPubkey[tx];
        outputCommitment[tx] === outCommitmentHasher[tx].out;

        // Increases sumOuts of the correct asset by outAmount
        for (var i = 0; i < nOutAssets; i++) {
            for (var j = 0; j < nAssets; j++) {
                sumOut[tx][i][j] = AND();
                sumOut[tx][i][j].a <== outAmount[tx][i];
                sumOut[tx][i][j].b <== outIndices[tx][i][j];
                sumOuts[j] += sumOut[tx][i][j].out;
            }
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

    // verify amount invariant
    sumIns[0] + publicAmountSol === sumOuts[0];
    sumIns[1] + publicAmountSpl === sumOuts[1];

    for (var a = 2; a < nAssets; a++) {
      sumIns[a] === sumOuts[a];
    }

    signal input internalTxIntegrityHash;
    // optional safety constraint to make sure txIntegrityHash cannot be changed
    internalTxIntegrityHash === txIntegrityHash;

}
