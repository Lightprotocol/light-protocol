pub const CONNECTING_HASH_VERIFIER_TWO: &str = "
    assert( nIns * nAssets < 49);
    assert( nInAssets <= nAssets);
    assert( nOutAssets <= nAssets);
    signal input publicProgramId;
    signal input publicTransactionHash;

    // privatePublicDataHash is not a public input just used to recompute the transaction hash
    signal input privatePublicDataHash;
    signal input inAmount[nIns][nInAssets];
    signal input inAsset[nIns][nInAssets];
    signal input inOwner[nIns];
    signal input inBlinding[nIns];
    signal input inDataHash[nIns];
    signal input inIndices[nIns][nInAssets][nAssets];
    signal input inMetaHash[nIns];
    signal input inAddress[nIns];

    // data for transaction outputsAccount
    signal input outAmount[nOuts][nOutAssets];
    signal input outAsset[nOuts][nOutAssets];
    signal input outOwner[nOuts];
    signal input outBlinding[nOuts];
    signal input outDataHash[nOuts];
    signal input outIndices[nOuts][nOutAssets][nAssets];
    signal input outMetaHash[nOuts];
    signal input outAddress[nOuts];

    signal input assetPublicKeys[nAssets];

    component transactionEnvironment = PspTransaction(nIns, nOuts, nAssets, nInAssets, nOutAssets);
    transactionEnvironment.inAmount <== inAmount;
    transactionEnvironment.inAsset <== inAsset;

    transactionEnvironment.inOwner <== inOwner;
    transactionEnvironment.inBlinding <== inBlinding;
    transactionEnvironment.inDataHash <== inDataHash;
    transactionEnvironment.outAmount <== outAmount;
    transactionEnvironment.outOwner <== outOwner;
    transactionEnvironment.outBlinding <== outBlinding;
    transactionEnvironment.outDataHash <== outDataHash;
    transactionEnvironment.outAsset <== outAsset;
    transactionEnvironment.inVersion <== 0;
    transactionEnvironment.outVersion <== 0;
    transactionEnvironment.inType <== 0;
    transactionEnvironment.outType <== 0;
    transactionEnvironment.privatePublicDataHash <== privatePublicDataHash;
    transactionEnvironment.publicProgramId <== publicProgramId;
    transactionEnvironment.publicTransactionHash <== publicTransactionHash;
    transactionEnvironment.inMetaHash <== inMetaHash;
    transactionEnvironment.outMetaHash <== outMetaHash;
    transactionEnvironment.inAddress <== inAddress;
    transactionEnvironment.outAddress <== outAddress;
";
