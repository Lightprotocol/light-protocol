pragma circom 2.1.4;
include "poseidon.circom";
include "gates.circom";
include "./checkIndexes.circom";

template UtxoSum(no, noAssets, nAssets) {
    signal input amount[no][noAssets];
    signal input indices[no][noAssets][nAssets];
    signal output sum[nAssets];
    component sumHelper[no][noAssets][nAssets];
    var sumHelperVar[nAssets];

    for (var i = 0; i < nAssets; i++) {
        sumHelperVar[i] = 0;
    }
    for (var tx = 0; tx < no; tx++) {
        for (var i = 0; i < noAssets; i++) {
            for (var j = 0; j < nAssets; j++) {
                sumHelper[tx][i][j] = AND();
                sumHelper[tx][i][j].a <== amount[tx][i];
                sumHelper[tx][i][j].b <== indices[tx][i][j];
                sumHelperVar[j] += sumHelper[tx][i][j].out;
            }
        }
    }
    sum <== sumHelperVar;
}

// add in and out version
// add in and out pooltype
// nOut, nAssets, nOutAssets
template CheckUtxoIntegrity(no, nAssets, nNoAssets) {
    signal input version;
    signal input type;
    signal input assetPublicKeys[nAssets];
    signal input indices[no][nNoAssets][nAssets];

    signal  input amount[no][nNoAssets];
    signal  input owner[no];
    signal  input blinding[no];
    signal  input dataHash[no];
    signal  input metaHash[no];
    signal  input address[no];
    signal output utxoHash[no];
    signal output positiveAmount[no];

    component checkIndices = CheckIndices(no,nNoAssets, nAssets);
    for (var i = 0; i < no; i++) {
        for (var j = 0; j < nNoAssets; j++) {
          checkIndices.amounts[i][j] <== amount[i][j];

          for(var z = 0; z < nAssets; z++) {
            checkIndices.indices[i][j][z] <== indices[i][j][z];
          }
        }
    }
    component getAsset[no][nNoAssets][nAssets];
    component utxoHasher[no];
    component amountCheck[no][nNoAssets];
    component amountsHasher[no];
    component assetsHasher[no];
    for (var tx = 0; tx < no; tx++) {
        // for every asset for every tx only one index is 1 others are 0
        // select the asset corresponding to the index
        // and add it to the assetHasher
        assetsHasher[tx] = Poseidon(nNoAssets);

        for (var a = 0; a < nNoAssets; a++) {
            var assetPublicKey = 0;

            for (var i = 0; i < nAssets; i++) {
                getAsset[tx][a][i] = AND();
                getAsset[tx][a][i].a <== assetPublicKeys[i];
                getAsset[tx][a][i].b <== indices[tx][a][i];
                assetPublicKey += getAsset[tx][a][i].out;
            }
            assetsHasher[tx].inputs[a] <== assetPublicKey;
        }

        for (var i = 0; i < nNoAssets; i++) {
            // Check that amount fits into 64 bits to prevent overflow
            amountCheck[tx][i] = Num2Bits(64);
            amountCheck[tx][i].in <== amount[tx][i];
        }
        var sumAmount = 0;
        amountsHasher[tx] = Poseidon(nNoAssets);
        for (var i = 0; i < nNoAssets; i++) {
            amountsHasher[tx].inputs[i] <== amount[tx][i];
            sumAmount += amount[tx][i];
        }
        positiveAmount[tx] <== sumAmount;
        utxoHasher[tx] = Poseidon(9);
        utxoHasher[tx].inputs[0] <== version;
        utxoHasher[tx].inputs[1] <== amountsHasher[tx].out;
        utxoHasher[tx].inputs[2] <== owner[tx];
        utxoHasher[tx].inputs[3] <== blinding[tx];
        utxoHasher[tx].inputs[4] <== assetsHasher[tx].out;
        utxoHasher[tx].inputs[5] <== dataHash[tx];
        utxoHasher[tx].inputs[6] <== type;
        utxoHasher[tx].inputs[7] <== metaHash[tx];
        utxoHasher[tx].inputs[8] <== address[tx];
        utxoHash[tx] <== utxoHasher[tx].out;
    }
}

template ComputeNullifier(no) {
    signal input inUtxoHash[no];
    signal input leafIndex[no];
    signal input inPrivateKey[no];
    signal output nullifier[no];

    component inSignature[no];
    component nullifierHasher[no];

    for (var tx = 0; tx < no; tx++) {
        inSignature[tx] = Signature();
        inSignature[tx].privateKey <== inPrivateKey[tx];
        inSignature[tx].commitment <== inUtxoHash[tx];
        inSignature[tx].merklePath <== leafIndex[tx];

        nullifierHasher[tx] = Poseidon(3);
        nullifierHasher[tx].inputs[0] <== inUtxoHash[tx];
        nullifierHasher[tx].inputs[1] <== leafIndex[tx];
        nullifierHasher[tx].inputs[2] <== inSignature[tx].out;
        nullifier[tx] <== nullifierHasher[tx].out;
    }
}

// check that there are no same value in the array
template UniquenessArray(no) {
    signal input array[no];
    component sameValue[no * (no - 1) / 2];
    var index = 0;
    for (var i = 0; i < no - 1; i++) {
      for (var j = i + 1; j < no; j++) {
          sameValue[index] = IsEqual();
          sameValue[index].in[0] <== array[i];
          sameValue[index].in[1] <== array[j];
          sameValue[index].out === 0;
          index++;
      }
    }
}

template UtxoHasher(no, nAssets) {
    signal input version;
    signal input asset[no][nAssets];
    signal input amount[no][nAssets];
    signal input owner[no];
    signal input blinding[no];
    signal input dataHash[no];
    signal input type;
    signal input address[no];
    signal input metaHash[no];

    signal output utxoHash[no];

    component assetHasher[no];
    for (var i = 0; i < no; i++) {
        assetHasher[i] = Poseidon(nAssets);
        for (var j = 0; j < nAssets; j++) {
            assetHasher[i].inputs[j] <== asset[i][j];
        }
    }

    component amountHasher[no];
    for (var i = 0; i < no; i++) {
        amountHasher[i] = Poseidon(nAssets);
        for (var j = 0; j < nAssets; j++) {
            amountHasher[i].inputs[j] <== amount[i][j];
        }
    }

    component hasher[no];
    for (var i = 0; i < no; i++) {
        hasher[i] = Poseidon(9);
        hasher[i].inputs[0] <== version;
        hasher[i].inputs[1] <== amountHasher[i].out;
        hasher[i].inputs[2] <== owner[i];
        hasher[i].inputs[3] <== blinding[i];
        hasher[i].inputs[4] <== assetHasher[i].out;
        hasher[i].inputs[5] <== dataHash[i];
        hasher[i].inputs[6] <== type;
        hasher[i].inputs[7] <== metaHash[i];
        hasher[i].inputs[8] <== address[i];
        utxoHash[i] <== hasher[i].out;
    }
}
