pragma circom 2.1.4;
include "poseidon.circom";
include "../merkle-tree/merkleProof.circom";
include "../transaction-utils/keypair.circom";
include "../transaction-utils/utxo.circom";
include "../transaction-utils/programChecks.circom";
include "gates.circom";

template PrivateTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets) {

    // public inputs, do not change the order!
    signal input publicRoot;
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;
    // nullifiers of in utxos
    signal input publicNullifier[nIns];
    // utxoHashes of out utxos
    signal input publicUtxoHash[nOuts];


    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;


    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inVerifierPublicKey[nIns];
    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];
    signal  input outVerifierPublicKey[nOuts];

    
    component transaction = PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets);
    transaction.root <== publicRoot;
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
    transaction.inVerifierPublicKey <== inVerifierPublicKey;
    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.publicUtxoHash <== publicUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;
    transaction.outVerifierPublicKey <== outVerifierPublicKey;
    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;

    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
        transaction.inOwner[inUtxo] <== inKeypair[inUtxo].publicKey;
    }

    // check that no in utxo is a program utxo
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inVerifierPublicKey[inUtxo] === 0;
        inDataHash[inUtxo] === 0;
    }
}

template PrivateProgramTransaction(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets) {

    // public inputs, do not change the order!
    signal input publicRoot;
    signal input publicAmountSpl;
    signal input publicDataHash;
    signal input publicAmountSol;
    signal input publicMintPublicKey;
    // nullifiers of in utxos
    signal input publicNullifier[nIns];
    // utxoHashes of out utxos
    signal input publicUtxoHash[nOuts];

    // program transaction specific public inputs
    signal input publicProgramId;
    signal input publicTransactionHash;

    // helper inputs
    signal input assetPublicKeys[nAssets];
    signal input privatePublicDataHash;


    // data to check in utxos
    signal input inAmount[nIns][nInAssets];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inVerifierPublicKey[nIns];
    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data to check out utxos
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];
    signal  input outVerifierPublicKey[nOuts];



    
    component transaction = PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets);
    transaction.root <== publicRoot;
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
    transaction.inVerifierPublicKey <== inVerifierPublicKey;
    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;
    transaction.publicUtxoHash <== publicUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;
    transaction.outVerifierPublicKey <== outVerifierPublicKey;
    transaction.assetPublicKeys <== assetPublicKeys;
    transaction.inVersion <== 0;
    transaction.outVersion <== 0;
    transaction.inType <== 0;
    transaction.outType <== 0;
    component inKeypair[nIns];
    for (var inUtxo = 0; inUtxo < nIns; inUtxo++) {
        inKeypair[inUtxo] = Keypair();
        inKeypair[inUtxo].privateKey <== inPrivateKey[inUtxo];
        transaction.inOwner[inUtxo] <== inKeypair[inUtxo].publicKey;
    }

    component checkProgramTransaction = CheckProgramTransaction(nIns, nOuts);
    checkProgramTransaction.publicProgramId <== publicProgramId;
    checkProgramTransaction.inVerifierPublicKey <== inVerifierPublicKey;
    checkProgramTransaction.inUtxoHash <== transaction.inUtxoHash;
    checkProgramTransaction.outUtxoHash <== transaction.outUtxoHash;
    checkProgramTransaction.publicDataHash <== publicDataHash;
    checkProgramTransaction.publicTransactionHash <== publicTransactionHash;
    
}

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
template TransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets) {

    // Range Check to prevent an overflow of wrong circuit instantiation
    assert( nIns * nAssets < 1000);
    assert( nInAssets <= nAssets);
    assert( nOutAssets <= nAssets);

    signal input root;
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

    signal input inAmount[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inVerifierPublicKey[nIns];
    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data for transaction out utxos
    signal input publicUtxoHash[nOuts];
    signal input outAmount[nOuts][nOutAssets];
    signal input outOwner[nOuts];
    signal input outBlinding[nOuts];
    signal input outDataHash[nOuts];
    signal input outIndices[nOuts][nOutAssets][nAssets];
    signal input outVerifierPublicKey[nOuts];

    // outputs
    signal output inUtxoHash[nIns];
    signal output outUtxoHash[nOuts];


    // feeAsset is asset indexFeeAsset
    assetPublicKeys[indexFeeAsset] === feeAsset;

    // If public amount is != 0 then check that assetPublicKeys[indexPublicAsset] == publicMintPublicKey
    // TODO: move public amount checks to a separate component and a level higher since we will have primarily compressed transactions
    component checkMintPublicKey = ForceEqualIfEnabled();
    checkMintPublicKey.in[0] <== assetPublicKeys[indexPublicAsset];
    checkMintPublicKey.in[1] <== publicMintPublicKey;

    checkMintPublicKey.enabled <== publicAmountSpl;

    component assetCheck[nAssets];
    for (var i = 0; i < nAssets; i++) {
        assetCheck[i] = Num2Bits(248);
        assetCheck[i].in <== assetPublicKeys[i];
    }

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
    checkInUtxos.verifierPublicKey <== inVerifierPublicKey;
    inUtxoHash <== checkInUtxos.utxoHash;

    component inTree[nIns];
    component inCheckRoot[nIns];
    for (var tx = 0; tx < nIns; tx++) {
        inTree[tx] = MerkleProof(levels);
        inTree[tx].leaf <== inUtxoHash[tx];
        inTree[tx].leafIndex <== leafIndex[tx];
        inTree[tx].pathElements <== merkleProof[tx];

        // check merkle proof only if amount is non-zero
        inCheckRoot[tx] = ForceEqualIfEnabled();
        inCheckRoot[tx].in[0] <== root;
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
    checkOutUtxos.verifierPublicKey <== outVerifierPublicKey;
    outUtxoHash <== checkOutUtxos.utxoHash;

    // check that out utxos are the same as public out utxoHashes
    publicUtxoHash === outUtxoHash;

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


template PrivateTransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets) {

    signal input root;
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


    signal input nullifier[nIns];
    signal input inAmount[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inPrivateKey[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inVerifierPublicKey[nIns];
    signal input leafIndex[nIns];
    signal input merkleProof[nIns][levels];
    signal input inIndices[nIns][nInAssets][nAssets];

    // data for transaction outputs
    signal  input publicUtxoHash[nOuts];
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outIndices[nOuts][nOutAssets][nAssets];
    signal  input outVerifierPublicKey[nOuts];

    signal output inUtxoHash[nIns];
    signal output outUtxoHash[nOuts];


    component transaction = TransactionLib(levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets);
    // public inputs
    transaction.root <== root;
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

    // in utxos
    transaction.inOwner <== inOwner;
    transaction.inAmount <== inAmount;
    transaction.inBlinding <== inBlinding;
    transaction.inDataHash <== inDataHash;
    transaction.inVerifierPublicKey <== inVerifierPublicKey;
    transaction.leafIndex <== leafIndex;
    transaction.merkleProof <== merkleProof;
    transaction.inIndices <== inIndices;

    // out utxos
    transaction.publicUtxoHash <== publicUtxoHash;
    transaction.outAmount <== outAmount;
    transaction.outOwner <== outOwner;
    transaction.outBlinding <== outBlinding;
    transaction.outDataHash <== outDataHash;
    transaction.outIndices <== outIndices;
    transaction.outVerifierPublicKey <== outVerifierPublicKey;

    inUtxoHash <== transaction.inUtxoHash;
    outUtxoHash <== transaction.outUtxoHash;

    // checks that nullfiers are correct
    component nullifiers = ComputeNullifier(nIns);
    nullifiers.inUtxoHash <== transaction.inUtxoHash;
    nullifiers.leafIndex <== leafIndex;
    nullifiers.inPrivateKey <== inPrivateKey;
    nullifier === nullifiers.nullifier;

    // check that input nullifiers are unique
    component uniquenessNullifiers = UniquenessArray(nIns);
    uniquenessNullifiers.array <== nullifier;
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
    signal input inVerifierPublicKey[nIns];

    // data for transaction outputs
    signal  input outAmount[nOuts][nOutAssets];
    signal  input outAsset[nOuts][nOutAssets];
    signal  input outOwner[nOuts];
    signal  input outBlinding[nOuts];
    signal  input outDataHash[nOuts];
    signal  input outVerifierPublicKey[nOuts];

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
    inUtxoHasher.verifierPublicKey <== inVerifierPublicKey;
    inputUtxoHash <== inUtxoHasher.utxoHash;

    component outUtxoHasher = UtxoHasher(nOuts, nOutAssets);
    outUtxoHasher.version <== outVersion;
    outUtxoHasher.asset <== outAsset;
    outUtxoHasher.amount <== outAmount;
    outUtxoHasher.owner <== outOwner;
    outUtxoHasher.blinding <== outBlinding;
    outUtxoHasher.dataHash <== outDataHash;
    outUtxoHasher.type <== outType;
    outUtxoHasher.verifierPublicKey <== outVerifierPublicKey;
    outputUtxoHash <== outUtxoHasher.utxoHash;

    component checkProgramTransaction = CheckProgramTransaction(nIns, nOuts);
    checkProgramTransaction.publicProgramId <== publicProgramId;
    checkProgramTransaction.inVerifierPublicKey <== inVerifierPublicKey;
    checkProgramTransaction.inUtxoHash <== inputUtxoHash;
    checkProgramTransaction.outUtxoHash <== outputUtxoHash;
    checkProgramTransaction.publicDataHash <== privatePublicDataHash;
    checkProgramTransaction.publicTransactionHash <== publicTransactionHash;
}