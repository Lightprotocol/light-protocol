/**
 *   it("getIndices", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let mockPubkey = SolanaKeypair.generate().publicKey;
    let lightProvider = await LightProvider.loadMock();

    let deposit_utxo1 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_1, BN_2],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: account.pubkey,
    });

    const relayer = new Relayer(mockPubkey, mockPubkey, new BN(5000));

    let params = new TransactionParameters({
      inputUtxos: [deposit_utxo1],
      eventMerkleTreePubkey: mockPubkey,
      transactionMerkleTreePubkey: mockPubkey,
      recipientSpl: mockPubkey,
      recipientSol: mockPubkey,
      poseidon,
      action: Action.UNSHIELD,
      relayer,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
      account,
    });

    let tx = new Transaction({
      provider: lightProvider,
      params,
    });

    const indices1 = tx.getIndices([deposit_utxo1]);
    assert.equal(indices1[0][0][0], "1");
    assert.equal(indices1[0][0][1], "0");
    assert.equal(indices1[0][0][2], "0");
    assert.equal(indices1[0][1][0], "0");
    assert.equal(indices1[0][1][1], "1");
    assert.equal(indices1[0][1][2], "0");

    const indices2 = tx.getIndices([deposit_utxo1, deposit_utxo1]);
    assert.equal(indices2[0][0][0], "1");
    assert.equal(indices2[0][0][1], "0");
    assert.equal(indices2[0][0][2], "0");
    assert.equal(indices2[0][1][0], "0");
    assert.equal(indices2[0][1][1], "1");
    assert.equal(indices2[0][1][2], "0");

    let deposit_utxo2 = new Utxo({
      poseidon,
      assets: [FEE_ASSET],
      amounts: [BN_1],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: new Account({ poseidon }).pubkey,
    });

    const indices3 = tx.getIndices([deposit_utxo2]);
    assert.equal(indices3[0][0][0], "1");
    assert.equal(indices3[0][0][1], "0");
    assert.equal(indices3[0][0][2], "0");
    assert.equal(indices3[0][1][0], "0");
    assert.equal(indices3[0][1][1], "0");
    assert.equal(indices3[0][1][2], "0");

    let deposit_utxo3 = new Utxo({
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: new Account({ poseidon }).pubkey,
    });

    const indices4 = tx.getIndices([deposit_utxo3]);
    assert.equal(indices4[0][0][0], "0");
    assert.equal(indices4[0][0][1], "0");
    assert.equal(indices4[0][0][2], "0");
    assert.equal(indices4[0][1][0], "0");
    assert.equal(indices4[0][1][1], "0");
    assert.equal(indices4[0][1][2], "0");

    let deposit_utxo4 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_0, BN_2],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: new Account({ poseidon }).pubkey,
    });

    const indices5 = tx.getIndices([deposit_utxo4]);
    assert.equal(indices5[0][0][0], "1");
    assert.equal(indices5[0][0][1], "0");
    assert.equal(indices5[0][0][2], "0");
    assert.equal(indices5[0][1][0], "0");
    assert.equal(indices5[0][1][1], "1");
    assert.equal(indices5[0][1][2], "0");

    const indices6 = tx.getIndices([deposit_utxo3, deposit_utxo4]);
    assert.equal(indices6[0][0][0], "0");
    assert.equal(indices6[0][0][1], "0");
    assert.equal(indices6[0][0][2], "0");
    assert.equal(indices6[0][1][0], "0");
    assert.equal(indices6[0][1][1], "0");
    assert.equal(indices6[0][1][2], "0");

    assert.equal(indices6[1][0][0], "1");
    assert.equal(indices6[1][0][1], "0");
    assert.equal(indices6[1][0][2], "0");
    assert.equal(indices6[1][1][0], "0");
    assert.equal(indices6[1][1][1], "1");
    assert.equal(indices6[1][1][2], "0");

    let deposit_utxo5 = new Utxo({
      poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [BN_2, BN_0],
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: new Account({ poseidon }).pubkey,
    });

    const indices7 = tx.getIndices([deposit_utxo5]);
    assert.equal(indices7[0][0][0], "1");
    assert.equal(indices7[0][0][1], "0");
    assert.equal(indices7[0][0][2], "0");
    assert.equal(indices7[0][1][0], "0");
    assert.equal(indices7[0][1][1], "1");
    assert.equal(indices7[0][1][2], "0");
  });
 */
