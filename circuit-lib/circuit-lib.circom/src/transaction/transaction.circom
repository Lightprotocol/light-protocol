pragma circom 2.1.4;
include "poseidon.circom";
include "../merkle-tree/merkleProof.circom";
include "../transaction-utils/keypair.circom";
include "../transaction-utils/utxo.circom";
include "../transaction-utils/programChecks.circom";
include "gates.circom";

template PublicTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {
    assert((higherUtxoNumber == nIns && nIns >= nOuts) || (higherUtxoNumber == nOuts && nOuts >= nIns));

    // public inputs, do not change the order!
    signal input publicStateRoot[nIns];
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicInUtxoHash[nIns];
    // utxoHashes of out utxos
    signal input publicOutUtxoHash[nOuts];

    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;
    signal input isInProgramUtxo[nIns];


    signal input inOwner[nIns];
    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    var inDataHash[nIns];
    for (var i = 0; i < nIns; i++) {
        inDataHash[i] = 0;
    }

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];

    var inAddress[higherUtxoNumber];
    var isInAddress[nIns];
    var metaHash[higherUtxoNumber];
    var isMetaHashUtxo[higherUtxoNumber];
    var isAddressUtxo[higherUtxoNumber];
    var isOutProgramUtxo[nOuts];

    for (var i = 0; i < higherUtxoNumber; i++) {
        inAddress[i] = 0;
        isAddressUtxo[i] = 0;
        isMetaHashUtxo[i] = 0;
        metaHash[i] = 0;
    }


    component transaction = PublicTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicAmountSpl <== 0;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== 0;
    transaction.publicDataHash <== publicDataHash;

    transaction.inAmount <== inAmount;
    transaction.inPrivateKey <== inPrivateKey;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;
    transaction.inAddress <== inAddress;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;
    transaction.address <== inAddress;
    transaction.metaHash <== metaHash;
    transaction.isInAddress <== isInAddress;
    transaction.isMetaHashUtxo <== isMetaHashUtxo;
    transaction.isAddressUtxo <== isAddressUtxo;
    transaction.isOutProgramUtxo <== isOutProgramUtxo;
    transaction.isInProgramUtxo <== isInProgramUtxo;
    transaction.publicInUtxoHash <== publicInUtxoHash;

    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
        transaction.inOwner[inUtxo] <== inKeypair[inUtxo].publicKey;
    }

    // check that no in utxo is a program utxo
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inDataHash[inUtxo] === 0;
    }
}

template PublicProgramTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {
    assert((higherUtxoNumber == nIns && nIns >= nOuts) || (higherUtxoNumber == nOuts && nOuts >= nIns));

    // public inputs, do not change the order!
    signal input publicStateRoot[nIns];
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;

    signal input publicInUtxoHash[nOuts];
    // utxoHashes of out utxos
    signal input publicOutUtxoHash[nOuts];
    signal input publicNewAddress[nOuts];
    signal input publicInUtxoDataHash[nIns];

    // program transaction specific public inputs
    signal input publicProgramId;
    signal input publicTransactionHash;

    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;
    signal input isInProgramUtxo[nIns];


    signal input inOwner[nIns];
    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];
    signal input nullifierLeafIndex[nIns];
    signal input nullifierMerkleProof[nIns][levels];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];

    signal input metaHash[higherUtxoNumber];
    signal input isMetaHashUtxo[higherUtxoNumber];

    signal input inAddress[nIns];

    signal input isInAddress[nIns];
    // There can only be as many new addresses as out utxos
    signal input isNewAddress[nOuts];

    signal input isOutProgramUtxo[nOuts];

    // logic is if there is a public new address there cannot be
    // there can only be as many utxos with address and meta data as the lower number of in and out utxos
    component checkAddress = CheckAndSelectAddress(nIns, nOuts,  higherUtxoNumber);
    checkAddress.publicNewAddress <== publicNewAddress;

    checkAddress.inAddress <== inAddress;
    checkAddress.isInAddress <== isInAddress;
    checkAddress.isNewAddress <== isNewAddress;


    component transaction = PublicTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicAmountSpl <== publicAmountSpl;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== publicMintPublicKey;
    transaction.publicDataHash <== publicDataHash;

    transaction.inAmount <== inAmount;
    transaction.inPrivateKey <== inPrivateKey;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;
    transaction.inOwner <== inOwner;
    transaction.inAddress <== inAddress;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;
    transaction.address <== checkAddress.address;
    transaction.metaHash <== metaHash;
    transaction.publicInUtxoHash <== publicInUtxoHash;
    transaction.isMetaHashUtxo <== isMetaHashUtxo;
    transaction.isAddressUtxo <== checkAddress.isAddressUtxo;
    transaction.isOutProgramUtxo <== isOutProgramUtxo;
    transaction.isInProgramUtxo <== isInProgramUtxo;
    transaction.isInAddress <== isInAddress;

    // add additional input of owner
    // if there is data assert that it is the same as the public program id
    // if there is no data assert that the owner is the same as inKeypair[inUtxo].publicKey
    // can I get around the second if?


    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
    }

    component checkProgramTransaction = EnabledProgramTransaction(nIns, nOuts);
    checkProgramTransaction.inUtxoHash <== transaction.inUtxoHash;
    checkProgramTransaction.outUtxoHash <== transaction.outUtxoHash;
    checkProgramTransaction.publicDataHash <== publicDataHash;
    checkProgramTransaction.publicTransactionHash <== publicTransactionHash;

    component checkOwner[nIns];
    component ownerOrProgram[nIns];

    // Checks that inOwner is either the programId or the owner of the keypair
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        // select owner or program
        ownerOrProgram[inUtxo] = Select();
        ownerOrProgram[inUtxo].a <== inKeypair[inUtxo].publicKey  * (1 - isInProgramUtxo[inUtxo]);
        ownerOrProgram[inUtxo].b <== publicProgramId * isInProgramUtxo[inUtxo];
        checkOwner[inUtxo] = ForceEqualIfEnabled();
        checkOwner[inUtxo].in[0] <== inOwner[inUtxo];
        checkOwner[inUtxo].in[1] <== ownerOrProgram[inUtxo].out;
        checkOwner[inUtxo].enabled <== inDataHash[inUtxo] * isInProgramUtxo[inUtxo];

        // is either zero or one
        (1 - isInProgramUtxo[inUtxo]) * isInProgramUtxo[inUtxo] === 0;

        // data hash is zero if isProgram utxo is zero, isInProgramUtxo is one if dataHash is not zero
        inDataHash[inUtxo] === inDataHash[inUtxo] * isInProgramUtxo[inUtxo];
    }

    // isMetaHashUtxo and isAddressUtxo is either zero or one, if there is a metaHash or address respectively
    for (var i = 0; i < higherUtxoNumber; i++) {
        (1 - isMetaHashUtxo[i]) * isMetaHashUtxo[i] === 0;
        (1 - checkAddress.isAddressUtxo[i]) * checkAddress.isAddressUtxo[i] === 0;
        metaHash[i] * isMetaHashUtxo[i] === metaHash[i];
        checkAddress.address[i] * checkAddress.isAddressUtxo[i] === checkAddress.address[i];
    }

    component optionInUtxoDataHash[nInAssets];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        optionInUtxoDataHash[inUtxo] = ForceEqualIfEnabled();
        optionInUtxoDataHash[inUtxo].in[0] <== transaction.inDataHash[inUtxo];
        optionInUtxoDataHash[inUtxo].in[1] <== publicInUtxoDataHash[inUtxo];
        optionInUtxoDataHash[inUtxo].enabled <== inDataHash[inUtxo];
    }
}

template PrivateTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {

    // public inputs, do not change the order!
    signal input publicStateRoot[nIns];
    signal input publicNullifierRoot[nIns];
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;
    // nullifiers of in utxos
    signal input publicNullifier[nIns];
    // utxoHashes of out utxos
    signal input publicOutUtxoHash[nOuts];


    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;
    signal input address[higherUtxoNumber];
    signal input metaHash[higherUtxoNumber];

    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];
    signal input nullifierLeafIndex[nIns];
    signal input nullifierMerkleProof[nIns][levels];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];

    component transaction = PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicNullifierRoot <== publicNullifierRoot;
    transaction.publicAmountSpl <== publicAmountSpl;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== publicMintPublicKey;
    transaction.publicDataHash <== publicDataHash;

    transaction.nullifier <== publicNullifier;
    transaction.inAmount <== inAmount;
    transaction.inPrivateKey <== inPrivateKey;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.nullifierLeafIndex <== nullifierLeafIndex;
    transaction.nullifierMerkleProof <== nullifierMerkleProof;
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;
    transaction.address <== address;
    transaction.metaHash <== metaHash;


    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
        transaction.inOwner[inUtxo] <== inKeypair[inUtxo].publicKey;
    }

    // check that no in utxo is a program utxo
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inDataHash[inUtxo] === 0;
    }
    // TODO: enable once we find the time to refactor the psp examples and template (only program should be able to insert program utxos they own so that the programs can trust the data)
    // check that no out utxo is a program utxo
    // for (var outUtxo = 0; outUtxo < nOuts; outUtxo++) {
    //     outDataHash[outUtxo] === 0;
    // }
}

template PrivateProgramTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {
    assert((higherUtxoNumber == nIns && nIns >= nOuts) || (higherUtxoNumber == nOuts && nOuts >= nIns));

    // public inputs, do not change the order!
    signal input publicStateRoot[nIns];
    signal input publicNullifierRoot[nIns];
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;

    // nullifiers of in utxos
    signal input publicNullifier[nIns];
    // utxoHashes of out utxos
    signal input publicOutUtxoHash[nOuts];
    signal input publicNewAddress[nOuts];

    // program transaction specific public inputs
    signal input publicProgramId;
    signal input publicTransactionHash;

    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;
    signal input isInProgramUtxo[nIns];


    signal input inOwner[nIns];
    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];
    signal input nullifierLeafIndex[nIns];
    signal input nullifierMerkleProof[nIns][levels];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];

    signal input metaHash[higherUtxoNumber];
    signal input inAddress[nIns];

    signal input isInAddress[nIns];
    signal input isNewAddress[nOuts];

    component checkAddress = CheckAndSelectAddress(nIns, nOuts, higherUtxoNumber);
    checkAddress.publicNewAddress <== publicNewAddress;
    checkAddress.inAddress <== inAddress;
    checkAddress.isInAddress <== isInAddress;
    checkAddress.isNewAddress <== isNewAddress;


    component transaction = PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicNullifierRoot <== publicNullifierRoot;
    transaction.publicAmountSpl <== publicAmountSpl;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== publicMintPublicKey;
    transaction.publicDataHash <== publicDataHash;

    transaction.nullifier <== publicNullifier;
    transaction.inAmount <== inAmount;
    transaction.inPrivateKey <== inPrivateKey;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;
    transaction.inOwner <== inOwner;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;
    transaction.address <== checkAddress.address;
    transaction.metaHash <== metaHash;

    transaction.nullifierLeafIndex <== nullifierLeafIndex;
    transaction.nullifierMerkleProof <== nullifierMerkleProof;

    // add additional input of owner
    // if there is data assert that it is the same as the public program id
    // if there is no data assert that the owner is the same as inKeypair[inUtxo].publicKey
    // can I get around the second if?


    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
    }

    component checkProgramTransaction = CheckProgramTransaction(nIns, nOuts);
    checkProgramTransaction.inUtxoHash <== transaction.inUtxoHash;
    checkProgramTransaction.outUtxoHash <== transaction.outUtxoHash;
    checkProgramTransaction.publicDataHash <== publicDataHash;
    checkProgramTransaction.publicTransactionHash <== publicTransactionHash;

    component checkOwner[nIns];
    component ownerOrProgram[nIns];
    // Checks that inOwner is either the programId or the owner of the keypair
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        // select owner or program
        ownerOrProgram[inUtxo] = Select();
        ownerOrProgram[inUtxo].a <== inKeypair[inUtxo].publicKey  * (1 - isInProgramUtxo[inUtxo]);
        ownerOrProgram[inUtxo].b <== publicProgramId * isInProgramUtxo[inUtxo];

        checkOwner[inUtxo] = ForceEqualIfEnabled();
        checkOwner[inUtxo].in[0] <== inOwner[inUtxo];
        checkOwner[inUtxo].in[1] <== ownerOrProgram[inUtxo].out;
        checkOwner[inUtxo].enabled <== inDataHash[inUtxo] * isInProgramUtxo[inUtxo];

        // is either zero or one
        (1 - isInProgramUtxo[inUtxo]) * isInProgramUtxo[inUtxo] === 0;

        // data hash is zero if isProgram utxo is zero, isInProgramUtxo is one if dataHash is not zero
        inDataHash[inUtxo] === inDataHash[inUtxo] * isInProgramUtxo[inUtxo];
    }
}

template Select() {
    signal input a;
    signal input b;
    signal output out;
    out <== a + b;
}

// check that either newAddres, inAddress or both are zero
template CheckAndSelectAddress(nIns, nOuts, higherUtxoNumber) {
    signal input publicNewAddress[nOuts];
    signal input inAddress[nIns];
    signal input isInAddress[nIns];
    signal input isNewAddress[nOuts];
    // there can be max nOut addresses nIns + newAddresses
    signal output address[nOuts];
    signal output isAddressUtxo[nOuts];
    for (var i = 0; i < nOuts; i++) {
        // is zero or one
        (1 - isInAddress[i]) * isInAddress[i] === 0;
        (1 - isNewAddress[i]) * isNewAddress[i] === 0;
        // one or both are zero
        (1 - (isNewAddress[i] + isInAddress[i])) * (isNewAddress[i] + isInAddress[i]) === 0;

        // if isInAddress is zero then inAddress must be zero
        inAddress[i] * isInAddress[i] === inAddress[i];
        publicNewAddress[i] * isNewAddress[i] === publicNewAddress[i];

        address[i] <== inAddress[i] + publicNewAddress[i];
        isAddressUtxo[i] <== isInAddress[i] + isNewAddress[i];
    }
    // need to pad address values with zeros in case there are more in utxos than out utxos
    for (var i = nOuts; i < higherUtxoNumber; i++) {
        address[i] <== 0;
    }
}

/*
Conditions:
- if theres is a dataHash then we need to check that the owner is equal to the publicProgramId
- if public metaHash input is 0 then we don't check that it is equal to the metaHash input of the utxo with the same index
- if public dataHash input is 0 then we don't check that it is equal to the metaHash input of the utxo with the same index
-
*/


/**
*
* pooltype consistency check
* transaction version consistency
*
* inUtxos
*     integrity & signer utxo check
*     inclusion for in utxos
*
* outUtxos
*     integrity check
*     utxoHash check vs public outputs
*
* amount_sum check for every asset
*/
template TransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {

    assert((higherUtxoNumber == nIns && nIns >= nOuts) || (higherUtxoNumber == nOuts && nOuts >= nIns));

    // Range Check to prevent an overflow of wrong circuit instantiation
    assert( nIns * nAssets < 1000);
    assert( nInAssets <= nAssets);
    assert( nOutAssets <= nAssets);

    signal input publicStateRoot[nIns];
    signal input publicAmountSol;
    signal input publicAmountSpl;
    signal input publicMintPublicKey;
    signal input publicDataHash;
    signal input privatePublicDataHash;

    signal input assetPublicKeys[nAssets];
    signal input inVersion;
    signal input inType;

    signal input outVersion;
    signal input outType;
    signal input metaHash[higherUtxoNumber];
    signal input address[higherUtxoNumber];

    signal input inAmount[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inAddress[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data for transaction out utxos
    signal input publicOutUtxoHash[nOuts];
    signal input outAmount[nOuts][nOutAssets];
    signal input outOwner[nOuts];
    signal input outBlinding[nOuts];
    signal input outDataHash[nOuts];
    signal input outIndices[nOuts][nOutAssets][nAssets];
    // outputs
    signal output inUtxoHash[nIns];
    signal output outUtxoHash[nOuts];
    signal output positiveAmount[nIns];

    component checkCorrectPaddingAddress = CheckCorrectPadding(nIns, nOuts, higherUtxoNumber);
    checkCorrectPaddingAddress.value <== address;
    component checkCorrectPaddingMetaHash = CheckCorrectPadding(nIns, nOuts, higherUtxoNumber);
    checkCorrectPaddingMetaHash.value <== metaHash;
    // feeAsset is asset indexFeeAsset
    assetPublicKeys[indexFeeAsset] === feeAsset;

    // If public amount is != 0 then check that assetPublicKeys[indexPublicAsset] == publicMintPublicKey
    // TODO: move public amount checks to a separate component and a level higher since we will have primarily compressed transactions
    component checkMintPublicKey = ForceEqualIfEnabled();
    checkMintPublicKey.in[0] <== assetPublicKeys[indexPublicAsset];
    checkMintPublicKey.in[1] <== publicMintPublicKey;

    checkMintPublicKey.enabled <== publicAmountSpl;

    // component assetCheck[nAssets];
    // for (var i = 0; i < nAssets; i++) {
    //     assetCheck[i] = Num2Bits(248);
    //     assetCheck[i].in <== assetPublicKeys[i];
    // }

    // verify correctness of transaction inputs
    component checkInUtxos = CheckUtxoIntegrity(nIns, nAssets, nInAssets);

    checkInUtxos.version <== inVersion;
    checkInUtxos.assetPublicKeys <== assetPublicKeys;
    checkInUtxos.indices <== inIndices;
    checkInUtxos.amount <== inAmount;
    checkInUtxos.owner <== inOwner;
    checkInUtxos.blinding <== inBlinding;
    checkInUtxos.dataHash <== inDataHash;
    checkInUtxos.type <== inType;
        for (var i = 0; i < nIns; i++) {
        checkInUtxos.metaHash[i] <== metaHash[i];
        checkInUtxos.address[i] <== inAddress[i];
    }
    inUtxoHash <== checkInUtxos.utxoHash;
    positiveAmount <== checkInUtxos.positiveAmount;

    component inTree[nIns];
    component inCheckRoot[nIns];
    for (var tx = 0; tx < nIns; tx++) {
        inTree[tx] = MerkleProof(levels);
        inTree[tx].leaf <== checkInUtxos.utxoHash[tx];
        inTree[tx].leafIndex <== leafIndex[tx];
        inTree[tx].pathElements <== merkleProof[tx];
        // check merkle proof only if amount is non-zero
        inCheckRoot[tx] = ForceEqualIfEnabled();
        inCheckRoot[tx].in[0] <== publicStateRoot[tx];
        inCheckRoot[tx].in[1] <== inTree[tx].root;
        inCheckRoot[tx].enabled <== checkInUtxos.positiveAmount[tx];
    }

    // verify correctness of transaction outputs
    component checkOutUtxos = CheckUtxoIntegrity(nOuts, nAssets, nOutAssets);

    checkOutUtxos.version <== outVersion;
    checkOutUtxos.assetPublicKeys <== assetPublicKeys;
    checkOutUtxos.indices <== outIndices;
    checkOutUtxos.amount <== outAmount;
    checkOutUtxos.owner <== outOwner;
    checkOutUtxos.blinding <== outBlinding;
    checkOutUtxos.dataHash <== outDataHash;
    checkOutUtxos.type <== outType;
    for (var i = 0; i < nOuts; i++) {
        checkOutUtxos.metaHash[i] <== metaHash[i];
        checkOutUtxos.address[i] <== address[i];
    }
    outUtxoHash <== checkOutUtxos.utxoHash;


    component utxoSumIn = UtxoSum(nIns, nInAssets, nAssets);
    utxoSumIn.amount <== inAmount;
    utxoSumIn.indices <== inIndices;

    component utxoSumOut = UtxoSum(nOuts, nOutAssets, nAssets);
    utxoSumOut.amount <== outAmount;
    utxoSumOut.indices <== outIndices;

    // verify amount invariant
    utxoSumIn.sum[0] + publicAmountSol === utxoSumOut.sum[0];
    utxoSumIn.sum[1] + publicAmountSpl === utxoSumOut.sum[1];

    for (var a = 2; a < nAssets; a++) {
      utxoSumIn.sum[a] === utxoSumOut.sum[a];
    }

    publicDataHash === privatePublicDataHash;
}

// check address and metaHash integrity by just using one input and using it for in and out utxos
// problem what if we have a different number of in and out utxos? -> we need to use zeros for address and metaHash in the larger one
// solution make the address and metaHash inputs arrays of size nIns and nOuts assert that the others are zero
template CheckCorrectPadding(nIns, nOuts, higherUtxoNumber) {
    signal input value[higherUtxoNumber];
    for (var i = nOuts; i < nIns; i++) {
        value[i] === 0;
    }
    for (var i = nIns; i < nOuts; i++) {
        value[i] === 0;
    }
}

template PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {

    signal input publicStateRoot[nIns];
    signal input publicNullifierRoot[nIns];
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input privatePublicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;

    signal input assetPublicKeys[nAssets];
    signal input inVersion;
    signal input inType;
    signal input outVersion;
    signal input outType;
    signal input address[higherUtxoNumber];
    signal input metaHash[higherUtxoNumber];

    signal input nullifier[nIns];
    signal input inAmount[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];
    signal input nullifierLeafIndex[nIns];
    signal input nullifierMerkleProof[nIns][levels];

    // data for transaction outputs
    signal  input publicOutUtxoHash[nOuts];
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];


    signal output inUtxoHash[nIns];
    signal output outUtxoHash[nOuts];
    signal output positiveAmount[nOuts];


    component transaction = TransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    // public inputs
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicAmountSpl <== publicAmountSpl;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicDataHash <== publicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== publicMintPublicKey;

    // helper inputs
    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== inVersion;
    transaction.outVersion <== outVersion;
    transaction.inType <== inType;
    transaction.outType <== outType;
    for (var i = 0; i <nIns; i++) {
        transaction.inAddress[i] <== 0;
    }
    transaction.address <== address;
    transaction.metaHash <== metaHash;

    // in utxos
    transaction.inOwner <== inOwner;
    transaction.inAmount <== inAmount;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;

    // out utxos
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    inUtxoHash <== transaction.inUtxoHash;
    outUtxoHash <== transaction.outUtxoHash;
    // check that out utxos are the same as public out utxoHashes
    publicOutUtxoHash === outUtxoHash;

    // TODO: add verification that nullifier is within non inclusion range
    component inNullifierTree[nIns];
    component inCheckNullifierRoot[nIns];
    for (var tx = 0; tx < nIns; tx++) {
        inNullifierTree[tx] = MerkleProof(levels);
        inNullifierTree[tx].leaf <== nullifier[tx];
        inNullifierTree[tx].leafIndex <== nullifierLeafIndex[tx];
        inNullifierTree[tx].pathElements <== nullifierMerkleProof[tx];

        // TODO: enable when we have nullifier trees
        // check merkle proof only if amount is non-zero
        // inCheckNullifierRoot[tx] = ForceEqualIfEnabled();
        // inCheckNullifierRoot[tx].in[0] <== publicNullifierRoot[tx];
        // inCheckNullifierRoot[tx].in[1] <== inNullifierTree[tx].root;
        // inCheckNullifierRoot[tx].enabled <== transaction.positiveAmount[tx];
    }

    // check that input nullifiers are unique
    component uniquenessNullifiers = UniquenessArray(nIns);
    uniquenessNullifiers.array <== nullifier;
}

template PublicTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber) {

    signal input publicStateRoot[nIns];
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input privatePublicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;

    signal input assetPublicKeys[nAssets];
    signal input inVersion;
    signal input inType;
    signal input outVersion;
    signal input outType;
    signal input address[higherUtxoNumber];
    signal input inAddress[nIns];
    signal input metaHash[higherUtxoNumber];

    signal input inAmount[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];

    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data for transaction outputs
    signal  input publicOutUtxoHash[nOuts];
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];
    signal input isMetaHashUtxo[higherUtxoNumber];
    signal input isAddressUtxo[higherUtxoNumber];
    signal input isOutProgramUtxo[nOuts];
    signal input isInProgramUtxo[nIns];
    signal input publicInUtxoHash[nIns];
    signal input isInAddress[nIns];

    signal output inUtxoHash[nIns];
    signal output outUtxoHash[nOuts];
    signal output positiveAmount[nOuts];


    component transaction = TransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets, higherUtxoNumber);
    // public inputs
    transaction.publicStateRoot <== publicStateRoot;
    transaction.publicAmountSpl <== publicAmountSpl;
    transaction.privatePublicDataHash <== privatePublicDataHash;
    transaction.publicDataHash <== publicDataHash;
    transaction.publicAmountSol <== publicAmountSol;
    transaction.publicMintPublicKey <== publicMintPublicKey;

    // helper inputs
    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== inVersion;
    transaction.outVersion <== outVersion;
    transaction.inType <== inType;
    transaction.outType <== outType;
    transaction.inAddress <== inAddress;
    transaction.address <== address;
    transaction.metaHash <== metaHash;

    // in utxos
    transaction.inOwner <== inOwner;
    transaction.inAmount <== inAmount;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;

    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;

    // out utxos
    transaction.publicOutUtxoHash <== publicOutUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;

    inUtxoHash <== transaction.inUtxoHash;
    outUtxoHash <== transaction.outUtxoHash;

    // check that input nullifiers are unique
    component uniquenessNullifiers = UniquenessArray(nIns);
    uniquenessNullifiers.array <== inUtxoHash;


    // isMetaHashUtxo and isAddressUtxo is either zero or one, if there is a metaHash or address respectively
    for (var i = 0; i < higherUtxoNumber; i++) {
        (1 - isMetaHashUtxo[i]) * isMetaHashUtxo[i] === 0;
        (1 - isAddressUtxo[i]) * isAddressUtxo[i] === 0;
        metaHash[i] * isMetaHashUtxo[i] === metaHash[i];
        address[i] * isAddressUtxo[i] === address[i];
    }

    // allow that utxoHash and outUtxoHash are zero if there is no data or amounts
    component optionInUtxoHash[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        optionInUtxoHash[inUtxo] = ForceEqualIfEnabled();
        optionInUtxoHash[inUtxo].in[0] <== transaction.inUtxoHash[inUtxo];
        optionInUtxoHash[inUtxo].in[1] <== publicInUtxoHash[inUtxo];
        optionInUtxoHash[inUtxo].enabled <== isInProgramUtxo[inUtxo] + inAmount[inUtxo][0] + inAmount[inUtxo][1] + isMetaHashUtxo[inUtxo] + isInAddress[inUtxo];
    }

    for (var i = 0; i < nOuts; i++) {
        (1 - isOutProgramUtxo[i]) * isOutProgramUtxo[i] === 0;
        outDataHash[i] * isOutProgramUtxo[i] === outDataHash[i];
    }

    component optionOutUtxoHash[nOuts];
    for (var outUtxo = 0; outUtxo < nOuts; outUtxo++) {
        optionOutUtxoHash[outUtxo] = ForceEqualIfEnabled();
        optionOutUtxoHash[outUtxo].in[0] <== transaction.outUtxoHash[outUtxo];
        optionOutUtxoHash[outUtxo].in[1] <== publicOutUtxoHash[outUtxo];
        optionOutUtxoHash[outUtxo].enabled <== isOutProgramUtxo[outUtxo] + outAmount[outUtxo][0] + outAmount[outUtxo][1] + isMetaHashUtxo[outUtxo] + isAddressUtxo[outUtxo];
    }
}

template PspTransaction(nIns, nOuts, nAssets, nInAssets, nOutAssets) {

    signal input publicProgramId;
    signal input publicTransactionHash;
    signal input privatePublicDataHash;
    signal input inType;
    signal input outType;
    signal input inVersion;
    signal input outVersion;



    signal input inAmount[nIns][nInAssets];
    signal input inAsset[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inMetaHash[nOuts];
    signal input inAddress[nOuts];

    // data for transaction outputs
    signal input outAmount[nOuts][nOutAssets];
    signal input outAsset[nOuts][nOutAssets];
    signal input outOwner[nOuts];
    signal input outBlinding[nOuts];
    signal input outDataHash[nOuts];
    signal input outMetaHash[nOuts];
    signal input outAddress[nOuts];

    signal output inputUtxoHash[nIns];
    signal output outputUtxoHash[nOuts];

    component inUtxoHasher = UtxoHasher(nIns, nInAssets);
    inUtxoHasher.version <== inVersion;
    inUtxoHasher.asset <== inAsset;
    inUtxoHasher.amount <== inAmount;
    inUtxoHasher.owner <== inOwner;
    inUtxoHasher.blinding <== inBlinding;
    inUtxoHasher.dataHash <== inDataHash;
    inUtxoHasher.type <== inType;
    inUtxoHasher.metaHash <== inMetaHash;
    inUtxoHasher.address <== inAddress;
    inputUtxoHash <== inUtxoHasher.utxoHash;

    component outUtxoHasher = UtxoHasher(nOuts, nOutAssets);
    outUtxoHasher.version <== outVersion;
    outUtxoHasher.asset <== outAsset;
    outUtxoHasher.amount <== outAmount;
    outUtxoHasher.owner <== outOwner;
    outUtxoHasher.blinding <== outBlinding;
    outUtxoHasher.dataHash <== outDataHash;
    outUtxoHasher.type <== outType;
    outUtxoHasher.metaHash <== outMetaHash;
    outUtxoHasher.address <== outAddress;
    outputUtxoHash <== outUtxoHasher.utxoHash;

    component checkProgramTransaction = CheckProgramTransaction(nIns, nOuts);
    checkProgramTransaction.inUtxoHash <== inputUtxoHash;
    checkProgramTransaction.outUtxoHash <== outputUtxoHash;
    checkProgramTransaction.publicDataHash <== privatePublicDataHash;
    checkProgramTransaction.publicTransactionHash <== publicTransactionHash;
}
