import * as anchor from "@coral-xyz/anchor";
const { SystemProgram } = require("@solana/web3.js");
const solana = require("@solana/web3.js");
import _ from "lodash";
import { assert } from "chai";
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  buildMerkleTree,
  MerkleTree,
  Transaction,
  VerifierZero,
  VerifierOne,
  Keypair,
  Utxo,
  newAccountWithLamports,
  newAccountWithTokens,
  executeUpdateMerkleTreeTransactions,
  executeMerkleTreeUpdateTransactions,
  createMintWrapper,
  getUninsertedLeaves,
  getInsertedLeaves,
  getUnspentUtxo,
  MerkleTreeConfig,
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
  FIELD_SIZE,
  ENCRYPTION_KEYPAIR,
  DEFAULT_PROGRAMS,
  setUpMerkleTree,
  initLookUpTableFromFile,
  testTransaction,
  hashAndTruncateToCircuit,
  MerkleTreeProgram,
  merkleTreeProgramId,
  MerkleTreeProgramIdl,
  MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  REGISTERED_POOL_PDA_SPL,
  REGISTERED_POOL_PDA_SOL,
  KEYPAIR_PRIVKEY,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_ONE_PDA,
  PRE_INSERTED_LEAVES_INDEX,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  AUTHORITY_ONE,
  TOKEN_AUTHORITY,
  MERKLE_TREE_AUTHORITY_PDA,
  USER_TOKEN_ACCOUNT,
  RECIPIENT_TOKEN_ACCOUNT,
  createTestAccounts,
  userTokenAccount,
  recipientTokenAccount,
  FEE_ASSET,
  VerifierProgramZero,
  verifierProgramZeroProgramId,
  confirmConfig
} from "../../light-sdk-ts/src/index";

import { BN } from "@coral-xyz/anchor";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER_RECIPIENT;

var SHIELDED_TRANSACTION;
var INVALID_SIGNER;
var INVALID_MERKLE_TREE_AUTHORITY_PDA;
var KEYPAIR;

// TODO: remove deprecated function calls
describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", confirmConfig); //anchor.getProvider();
  console.timeEnd("init provider");
  const merkleTreeProgram: anchor.Program<MerkleTreeProgramIdl> =
    new anchor.Program(MerkleTreeProgram, merkleTreeProgramId);

  it("init pubkeys ", async () => {
    await createTestAccounts(provider.connection);
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new Keypair({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
    RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    console.log("USER_TOKEN_ACCOUNT ", USER_TOKEN_ACCOUNT.publicKey.toBase58());
    console.log(
      "RECIPIENT_TOKEN_ACCOUNT ",
      RECIPIENT_TOKEN_ACCOUNT.publicKey.toBase58()
    );

    console.log("MERKLE_TREE_KEY ", MERKLE_TREE_KEY.toBase58());
    console.log("REGISTERED_VERIFIER_PDA ", REGISTERED_VERIFIER_PDA.toBase58());
    console.log(
      "REGISTERED_VERIFIER_ONE_PDA ",
      REGISTERED_VERIFIER_ONE_PDA.toBase58()
    );
    console.log("AUTHORITY ", AUTHORITY.toBase58());
    console.log("AUTHORITY_ONE ", AUTHORITY_ONE.toBase58());
    console.log(
      "PRE_INSERTED_LEAVES_INDEX ",
      PRE_INSERTED_LEAVES_INDEX.toBase58()
    );
    console.log("TOKEN_AUTHORITY ", TOKEN_AUTHORITY.toBase58());
    console.log("REGISTERED_POOL_PDA_SPL ", REGISTERED_POOL_PDA_SPL.toBase58());
    console.log(
      "REGISTERED_POOL_PDA_SPL_TOKEN ",
      REGISTERED_POOL_PDA_SPL_TOKEN.toBase58()
    );
    console.log("REGISTERED_POOL_PDA_SOL ", REGISTERED_POOL_PDA_SOL.toBase58());
    console.log(
      "MERKLE_TREE_AUTHORITY_PDA ",
      MERKLE_TREE_AUTHORITY_PDA.toBase58()
    );
  });

  it("Initialize Merkle Tree", async () => {
    await setUpMerkleTree(provider);
  });

  it.skip("Initialize Merkle Tree Test", async () => {
    const verifierProgramZero = new anchor.Program(
      VerifierProgramZero,
      verifierProgramZeroProgramId
    );
    // const verifierProgramOne = new anchor.Program(VerifierProgramOne, verifierProgramOneProgramId);

    // Security Claims
    // Init authority pda
    // - can only be inited by a hardcoded pubkey
    // Update authority pda
    // - can only be invoked by current authority
    //
    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY
    );
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    INVALID_SIGNER = new anchor.web3.Account();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        INVALID_SIGNER.publicKey,
        1_000_000_000_000
      ),
      "confirmed"
    );

    INVALID_MERKLE_TREE_AUTHORITY_PDA = (
      await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")],
        merkleTreeProgram.programId
      )
    )[0];
    let merkleTreeConfig = new MerkleTreeConfig({
      merkleTreePubkey: MERKLE_TREE_KEY,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    let error;

    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.initMerkleTreeAuthority();
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log(error);

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
      )
    );
    error = undefined;

    // init merkle tree with invalid signer
    try {
      await merkleTreeConfig.initMerkleTreeAuthority(INVALID_SIGNER);
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;

    // initing real mt authority
    await merkleTreeConfig.initMerkleTreeAuthority();
    await merkleTreeConfig.initializeNewMerkleTree();

    let newAuthority = new anchor.web3.Account();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        newAuthority.publicKey,
        1_000_000_000_000
      ),
      "confirmed"
    );

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(
        newAuthority.publicKey,
        true
      );
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(
        newAuthority.publicKey,
        true
      );
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
    merkleTreeConfig.payer = newAuthority;
    await merkleTreeConfig.updateMerkleTreeAuthority(
      ADMIN_AUTH_KEYPAIR.publicKey
    );
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid pda
    let tmp = merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda;
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda =
      INVALID_SIGNER.publicKey;
    try {
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.origin, "registered_verifier_pda");
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda = tmp;
    error = undefined;

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.enableNfts(true);
    } catch (e) {
      error = e;
    }
    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.enableNfts(true);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    await merkleTreeConfig.enableNfts(true);

    let merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda
      );
    assert.equal(merkleTreeAuthority.enableNfts, true);
    await merkleTreeConfig.enableNfts(false);
    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda
      );
    assert.equal(merkleTreeAuthority.enableNfts, false);

    // update lock duration with invalid signer
    console.log("here");

    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateLockDuration(123);
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.updateLockDuration(123);
    } catch (e) {
      error = e;
    }

    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    await merkleTreeConfig.updateLockDuration(123);

    await merkleTreeConfig.updateLockDuration(10);

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, true);
    await merkleTreeConfig.enablePermissionlessSplTokens(false);
    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, false);

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));

    let registeredPoolTypePdaAccount =
      await merkleTreeProgram.account.registeredPoolType.fetch(
        merkleTreeConfig.poolTypes[0].poolPda
      );

    assert.equal(
      registeredPoolTypePdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString()
    );

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    // valid
    await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));

    let registeredSolPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        merkleTreeConfig.poolPdas[0].pda
      );

    assert.equal(
      registeredSolPdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString()
    );
    assert.equal(registeredSolPdaAccount.index, 0);
    assert.equal(
      registeredSolPdaAccount.assetPoolPubkey.toBase58(),
      merkleTreeConfig.poolPdas[0].pda.toBase58()
    );

    let mint = await createMintWrapper({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized"
    );
    error = undefined;

    // valid
    await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    console.log(merkleTreeConfig.poolPdas);

    let registeredSplPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        merkleTreeConfig.poolPdas[0].pda
      );
    registeredSplPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda
      );

    console.log(registeredSplPdaAccount);

    assert.equal(
      registeredSplPdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString()
    );
    assert.equal(registeredSplPdaAccount.index.toString(), "1");
    assert.equal(
      registeredSplPdaAccount.assetPoolPubkey.toBase58(),
      merkleTreeConfig.poolPdas[
        merkleTreeConfig.poolPdas.length - 1
      ].token.toBase58()
    );

    let merkleTreeAuthority1 =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda
      );
    console.log(merkleTreeAuthority1);
    assert.equal(merkleTreeAuthority1.registeredAssetIndex.toString(), "2");
    // let nftMint = await createMintWrapper({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true, connection: provider.connection})

    // var userTokenAccount = (await newAccountWithTokens({
    //   connection: provider.connection,
    //   MINT: nftMint,
    //   ADMIN_AUTH_KEYPAIR,
    //   userAccount: new anchor.web3.Account(),
    //   amount: 1
    // }))
  });

  it("Init Address Lookup Table", async () => {
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
  });

  it.skip("Deposit 10 utxo", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    for (var i = 0; i < 1; i++) {
      console.log("Deposit with 10 utxos ", i);

      let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

      console.log("MINT: ", MINT);
      console.log("ADMIN_AUTH_KEYPAIR: ", ADMIN_AUTH_KEYPAIR);
      console.log("depositAmount: ", depositAmount);

      await token.approve(
        provider.connection,
        ADMIN_AUTH_KEYPAIR,
        userTokenAccount,
        AUTHORITY_ONE, //delegate
        USER_TOKEN_ACCOUNT, // owner
        depositAmount * 2,
        [USER_TOKEN_ACCOUNT]
      );

      SHIELDED_TRANSACTION = new Transaction({
        // four static config fields
        payer: ADMIN_AUTH_KEYPAIR,
        encryptionKeypair: ENCRYPTION_KEYPAIR,

        // four static config fields
        merkleTree: new MerkleTree(18, POSEIDON),
        provider,
        lookupTable: LOOK_UP_TABLE,

        relayerRecipient: ADMIN_AUTH_KEYPAIR.publicKey,

        verifier: new VerifierOne(),
        shuffleEnabled: false,
        poseidon: POSEIDON,
      });

      let inputUtxos = [
        new Utxo({ poseidon: POSEIDON }),
        new Utxo({ poseidon: POSEIDON }),
        new Utxo({ poseidon: POSEIDON }),
        new Utxo({ poseidon: POSEIDON }),
      ];
      let deposit_utxo1 = new Utxo({
        poseidon: POSEIDON,
        assets: [FEE_ASSET, MINT],
        amounts: [
          new anchor.BN(depositFeeAmount),
          new anchor.BN(depositAmount),
        ],
        keypair: KEYPAIR,
      });

      let outputUtxos = [deposit_utxo1];

      // await SHIELDED_TRANSACTION.compileTransaction({
      //   inputUtxos,
      //   outputUtxos,
      //   action: "DEPOSIT",
      //   assetPubkeys: [FEE_ASSET, hashAndTruncateToCircuit(MINT.toBytes())],
      //   relayerFee: new anchor.BN('0'),
      //   mintPubkey: hashAndTruncateToCircuit(MINT.toBytes()),
      //   sender: userTokenAccount,
      //   merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN
      // });
      await SHIELDED_TRANSACTION.compileTransaction({
        inputUtxos,
        outputUtxos,
        action: "DEPOSIT",
        assetPubkeys: [new BN(0), hashAndTruncateToCircuit(MINT.toBytes())],
        relayerFee: 0,
        sender: userTokenAccount,
        merkleTreeAssetPubkey: REGISTERED_POOL_PDA_SPL_TOKEN,
        config: { in: 10, out: 2 },
      });

      await SHIELDED_TRANSACTION.getProof();

      console.log("testTransaction Doesn't work");
      // await testTransaction({transaction: SHIELDED_TRANSACTION, deposit: true, enabledSignerTest: false, provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

      try {
        let res = await SHIELDED_TRANSACTION.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      try {
        await SHIELDED_TRANSACTION.checkBalances();
      } catch (e) {
        console.log(e);
      }
    }
  });
  const { keccak_256 } = require("@noble/hashes/sha3");

  it.skip("test styff", async () => {
    let x = [26,85,237,89,32,60,200,105,191,173,83,2,152,108,81,81,105,177,85,19,25,58,118,131,241,107,144,177,40,81,52,120,206,102,250,25,143,73,78,24,82,131,173,135,94,96,131,220,102,137,81,59,124,164,50,189,80,246,176,184,229,150,210,35,2,99,226,251,88,66,92,33,25,216,211,185,112,203,212,238,105,144,72,121,176,253,106,168,115,158,154,188,62,255,166,81,0,0,0,0,0,0,0,0,116,187,181,76,137,251,61,85,163,244,85,53,111,94,147,212,56,79,94,59,55,111,116,172,219,199,253,252,193,131,170,226,4,172,126,198,203,197,252,136,194,22,180,74,114,101,226,58,12,143,236,27,252,146,140,230,251,254,189,71,119,5,102,72,236,64,21,9,96,228,40,179,17,2,105,234,40,10,42,28,27,199,198,52,14,109,240,115,24,8,99,251,92,0,85,112,125,150,67,194,216,146,59,205,189,77,90,20,196,15,47,204,95,62,69,42,14,32,43,105,212,14,230,241,88,8,45,39,194,50,16,196,235,11,13,243,61,236,97,91,250,225,77,220,231,72,198,199,80,36,23,198,175,114,135,67,215,134,189,77,109,167,59,96,179,128,85,239,201,82,59,245,114,160,26,142,67,103,237,131,122,16,184,168,14,16,84,197,132,96,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    let y = [26, 85, 237, 89, 32, 60, 200, 105, 191, 173, 83, 2, 152, 108, 81, 81, 105, 177, 85, 19, 25, 58, 118, 131, 241, 107, 144, 177, 40, 81, 52, 120, 206, 102, 250, 25, 143, 73, 78, 24, 82, 131, 173, 135, 94, 96, 131, 220, 102, 137, 81, 59, 124, 164, 50, 189, 80, 246, 176, 184, 229, 150, 210, 35, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81, 0, 0, 0, 0, 0, 0, 0, 0, 116, 187, 181, 76, 137, 251, 61, 85, 163, 244, 85, 53, 111, 94, 147, 212, 56, 79, 94, 59, 55, 111, 116, 172, 219, 199, 253, 252, 193, 131, 170, 226, 4, 172, 126, 198, 203, 197, 252, 136, 194, 22, 180, 74, 114, 101, 226, 58, 12, 143, 236, 27, 252, 146, 140, 230, 251, 254, 189, 71, 119, 5, 102, 72, 236, 64, 21, 9, 96, 228, 40, 179, 17, 2, 105, 234, 40, 10, 42, 28, 27, 199, 198, 52, 14, 109, 240, 115, 24, 8, 99, 251, 92, 0, 85, 112, 125, 150, 67, 194, 216, 146, 59, 205, 189, 77, 90, 20, 196, 15, 47, 204, 95, 62, 69, 42, 14, 32, 43, 105, 212, 14, 230, 241, 88, 8, 45, 39, 194, 50, 16, 196, 235, 11, 13, 243, 61, 236, 97, 91, 250, 225, 77, 220, 231, 72, 198, 199, 80, 36, 23, 198, 175, 114, 135, 67, 215, 134, 189, 77, 109, 167, 59, 96, 179, 128, 85, 239, 201, 82, 59, 245, 114, 160, 26, 142, 67, 103, 237, 131, 122, 16, 184, 168, 14, 16, 84, 197, 132, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    for (var i = 0; i < y.length; i++) {

      if (x[i] != y[i]) {
        console.log(Array.from(x.slice(i -10, i+10)));
        console.log(Array.from(y.slice(i -10, i+10)));

        console.log(i);
        break
        
      }
    }

    const hash = keccak_256
      .create({ dkLen: 32 })
      .update(Buffer.from(x))
      .digest();
    console.log("extDataHash ", new anchor.BN(hash).mod(FIELD_SIZE).toArray("be", 32));
    const hash1 = keccak_256
      .create({ dkLen: 32 })
      .update(Buffer.from(y))
      .digest();
    console.log("extDataHash ", new anchor.BN(hash1).mod(FIELD_SIZE).toArray("be", 32));
  })
  var deposit_utxo1;
  it("Deposit", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    let depositAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    let depositFeeAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    try {
      await token.approve(
        provider.connection,
        ADMIN_AUTH_KEYPAIR,
        userTokenAccount,
        AUTHORITY, //delegate
        USER_TOKEN_ACCOUNT, // owner
        depositAmount * 2,
        [USER_TOKEN_ACCOUNT]
      );
    } catch (error) {
      console.log(error);
    }

    for (var i = 0; i < 1; i++) {
      console.log("Deposit ", i);

      SHIELDED_TRANSACTION = new Transaction({
        payer: ADMIN_AUTH_KEYPAIR,
        encryptionKeypair: ENCRYPTION_KEYPAIR,

        // four static config fields
        merkleTree: new MerkleTree(18, POSEIDON),
        provider,
        lookupTable: LOOK_UP_TABLE,

        relayerRecipient: ADMIN_AUTH_KEYPAIR.publicKey,

        verifier: new VerifierZero(),
        shuffleEnabled: false,
        poseidon: POSEIDON,
      });

      deposit_utxo1 = new Utxo({
        poseidon: POSEIDON,
        assets: [FEE_ASSET, MINT],
        amounts: [
          new anchor.BN(depositFeeAmount),
          new anchor.BN(depositAmount),
        ],
        keypair: KEYPAIR,
      });

      let outputUtxos = [deposit_utxo1];
      console.log(
        "outputUtxos[0].assetsCircuit[1]: ",
        outputUtxos[0].assetsCircuit[1]
      );

      await SHIELDED_TRANSACTION.compileTransaction({
        inputUtxos: [],
        outputUtxos,
        action: "DEPOSIT",
        assetPubkeys: [new anchor.BN(0), outputUtxos[0].assetsCircuit[1]],
        relayerFee: 0,
        sender: userTokenAccount,
        mintPubkey: hashAndTruncateToCircuit(MINT.toBytes()),
        merkleTreeAssetPubkey: REGISTERED_POOL_PDA_SPL_TOKEN,
        config: { in: 2, out: 2 },
      });

      await SHIELDED_TRANSACTION.getProof();

      // await testTransaction({transaction: SHIELDED_TRANSACTION, provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

      try {
        let res = await SHIELDED_TRANSACTION.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      try {
        await SHIELDED_TRANSACTION.checkBalances();
      } catch (e) {
        console.log(e);
      }
    }
  });

  it("Update Merkle Tree after Deposit", async () => {
    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY
    );

    // fetch uninserted utxos from chain
    let leavesPdas = await getUninsertedLeaves({
      merkleTreeProgram,
      merkleTreeIndex: mtFetched.nextIndex,
      connection: provider.connection,
    });

    let poseidon = await circomlibjs.buildPoseidonOpt();
    // build tree from chain
    let mtPrior = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: poseidon,
    });

    await executeUpdateMerkleTreeTransactions({
      connection: provider.connection,
      signer: ADMIN_AUTH_KEYPAIR,
      merkleTreeProgram: merkleTreeProgram,
      leavesPdas: leavesPdas.slice(0, 1),
      merkleTree: mtPrior,
      merkle_tree_pubkey: MERKLE_TREE_KEY,
      provider,
    });
    let mtAfter = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY
    );

    let merkleTree = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: POSEIDON,
    });
    //check correct insert
    assert.equal(
      new anchor.BN(
        mtAfter.roots[mtAfter.currentRootIndex],
        undefined,
        "le"
      ).toString(),
      merkleTree.root()
    );
  });

  it.skip("Update Merkle Tree Test", async () => {
    // Security Claims
    // CreateUpdateState
    // 1 leaves can only be inserted in the correct index order
    // 2 leaves cannot be inserted twice
    // 3 leaves are queued for a specific tree and can only be inserted in that tree
    // 4 lock is taken and cannot be taken again before expiry
    // 5 Merkle tree is registered
    //
    // Update
    // 6 signer is consistent
    // 7 is locked by update state account
    // 8 merkle tree is consistent
    //
    // Last Tx
    // 9 same leaves as in first tx are marked as inserted
    // 10 is in correct state
    // 11 is locked by update state account
    // 12 merkle tree is consistent
    // 13 signer is consistent

    const signer = ADMIN_AUTH_KEYPAIR;

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY
    );
    let error;

    // fetch uninserted utxos from chain
    let leavesPdas = await getUninsertedLeaves({
      merkleTreeProgram,
      merkleTreeIndex: mtFetched.nextIndex,
      connection: provider.connection,
    });

    let poseidon = await circomlibjs.buildPoseidonOpt();
    // build tree from chain
    let merkleTreeWithdrawal = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: poseidon,
    });

    let merkleTreeUpdateState = (
      await solana.PublicKey.findProgramAddress(
        [
          Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
          anchor.utils.bytes.utf8.encode("storage"),
        ],
        merkleTreeProgram.programId
      )
    )[0];
    let merkle_tree_pubkey = MERKLE_TREE_KEY;
    let connection = provider.connection;

    if (leavesPdas.length > 1) {
      // test leaves with higher starting index than merkle tree next index
      leavesPdas.reverse();
      try {
        const tx1 = await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: DEFAULT_PROGRAMS.systemProgram,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTree: merkle_tree_pubkey,
          })
          .remainingAccounts(leavesPdas)
          .preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({
              units: 1_400_000,
            }),
          ])
          .signers([signer])
          .rpc(confirmConfig);
        console.log("success 0");
      } catch (e) {
        error = e;
      }
      assert(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");

      leavesPdas.reverse();
      assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);

      console.log("Test property: 1");
      // Test property: 1
      // try with one leavespda of higher index
      try {
        const tx1 = await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTree: merkle_tree_pubkey,
          })
          .remainingAccounts(leavesPdas[1])
          .preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({
              units: 1_400_000,
            }),
          ])
          .signers([signer])
          .rpc(confirmConfig);
        console.log("success 1");
      } catch (e) {
        console.log(e);
        error = e;
      }
      assert(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");

      assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);
    } else {
      console.log("pdas.length <=" + 1 + " skipping some tests");
    }

    // Test property: 3
    // try with different Merkle tree than leaves are queued for
    // index might be broken it is wasn't set to mut didn't update
    let merkleTreeConfig = new MerkleTreeConfig({
      merkleTreePubkey: MERKLE_TREE_KEY,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });
    let different_merkle_tree = (
      await solana.PublicKey.findProgramAddress(
        [
          merkleTreeProgram.programId.toBuffer(),
          new anchor.BN(1).toArray("le", 8),
        ],
        merkleTreeProgram.programId
      )
    )[0];
    if ((await connection.getAccountInfo(different_merkle_tree)) == null) {
      await merkleTreeConfig.initializeNewMerkleTree(different_merkle_tree);
      console.log("created new merkle tree");
    }

    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: different_merkle_tree,
        })
        .remainingAccounts(leavesPdas)
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
      console.log("success 3");
    } catch (e) {
      console.log(e);
      error = e;
    }
    assert(error.error.errorCode.code == "LeavesOfWrongTree");
    assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);
    error = undefined;

    // correct
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
      console.log(error);
    }
    // should not be an error
    assert(error === undefined);
    console.log("created update state ", merkleTreeUpdateState.toBase58());

    assert((await connection.getAccountInfo(merkleTreeUpdateState)) != null);

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      MerkleTree: merkle_tree_pubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 1,
      merkleTreeProgram,
    });
    console.log("executeMerkleTreeUpdateTransactions 10");

    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      merkle_tree_pubkey,
      provider,
      merkleTreeUpdateState,
      numberOfTransactions: 10,
    });
    console.log("checkMerkleTreeUpdateStateCreated 22");

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      MerkleTree: merkle_tree_pubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 22, // 22 becaue one tx executes two instructions, it started out in ix index 1 and increments at the end of a tx
      merkleTreeProgram,
    });

    // Test property: 6
    // trying to use merkleTreeUpdateState with different signer

    let maliciousSigner = await newAccountWithLamports(provider.connection);
    console.log("maliciousSigner: ", maliciousSigner.publicKey.toBase58());

    let maliciousMerkleTreeUpdateState =
      solana.PublicKey.findProgramAddressSync(
        [
          Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())),
          anchor.utils.bytes.utf8.encode("storage"),
        ],
        merkleTreeProgram.programId
      )[0];
    let s = false;
    error = await executeMerkleTreeUpdateTransactions({
      signer: maliciousSigner,
      merkleTreeProgram,
      merkle_tree_pubkey,
      provider,
      merkleTreeUpdateState,
      numberOfTransactions: 1,
    });
    console.log(error);

    assert(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority."
      )
    );

    // Test property: 4
    // try to take lock
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: maliciousSigner.publicKey,
          merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .signers([maliciousSigner])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
      console.log(e);
    }
    assert(error.error.errorCode.code == "ContractStillLocked");

    // Test property: 10
    // try insert root before completing update transaction
    try {
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey,
        })
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "MerkleTreeUpdateNotInRootInsert");

    // sending additional tx to finish the merkle tree update
    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      merkle_tree_pubkey,
      provider,
      merkleTreeUpdateState,
      numberOfTransactions: 50,
    });

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      MerkleTree: merkle_tree_pubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 56,
      merkleTreeProgram,
    });

    // Test property: 11
    // final tx to insert root different UNREGISTERED_MERKLE_TREE
    try {
      console.log("final tx to insert root into different_merkle_tree");
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: different_merkle_tree,
        })
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "ContractStillLocked");

    // Test property: 13
    // final tx to insert root different signer
    try {
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: maliciousSigner.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey,
        })
        .signers([maliciousSigner])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "InvalidAuthority");

    var merkleTreeAccountPrior =
      await merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey);

    let merkleTree = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: poseidon,
    });

    // insert correctly
    await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey,
      })
      .signers([signer])
      .rpc(confirmConfig);
    console.log("merkleTreeUpdateState ", merkleTreeUpdateState);
    console.log("merkleTreeAccountPrior ", merkleTreeAccountPrior);
    console.log("leavesPdas[0] ", leavesPdas[0]);
    console.log("merkleTree ", merkleTree);
    console.log("merkle_tree_pubkey ", merkle_tree_pubkey);

    await checkMerkleTreeBatchUpdateSuccess({
      connection: provider.connection,
      merkleTreeUpdateState: merkleTreeUpdateState,
      merkleTreeAccountPrior,
      numberOfLeaves: 2,
      leavesPdas: [leavesPdas[0]],
      merkleTree: merkleTree,
      merkle_tree_pubkey: merkle_tree_pubkey,
      merkleTreeProgram,
    });

    console.log("Test property: 2");

    // Test property: 2
    // try to reinsert leavesPdas[0]
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "LeafAlreadyInserted");
  });

  // only works at the first try because the tests takes utxo in pos 0
  it("Withdraw", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY
    );
    let merkleTree = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: POSEIDON,
    });

    // get inserted leaves
    let leavesPdas = await getInsertedLeaves({
      merkleTreeProgram,
      merkleTreeIndex: mtFetched.nextIndex,
      connection: provider.connection,
    });

    let decryptedUtxo1 = await getUnspentUtxo(
      leavesPdas,
      provider,
      ENCRYPTION_KEYPAIR,
      KEYPAIR,
      FEE_ASSET,
      MINT,
      POSEIDON,
      merkleTreeProgram
    );
    decryptedUtxo1.getCommitment();

    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    SHIELDED_TRANSACTION = new Transaction({
      payer: ADMIN_AUTH_KEYPAIR,
      encryptionKeypair: ENCRYPTION_KEYPAIR,

      // four static config fields
      merkleTree,
      provider,
      lookupTable: LOOK_UP_TABLE,

      relayerRecipient: ADMIN_AUTH_KEYPAIR.publicKey,

      verifier: new VerifierZero(),
      shuffleEnabled: false,
      poseidon: POSEIDON,
    });

    let outputUtxos = [];

    let utxoIndex = 0;

    let inputUtxos = [];
    inputUtxos.push(decryptedUtxo1);

    assert(
      hashAndTruncateToCircuit(MINT.toBytes()).toString() ===
        inputUtxos[0].assetsCircuit[1].toString(),
      "inputUtxos[1] asset werid"
    );

    await SHIELDED_TRANSACTION.compileTransaction({
      inputUtxos: inputUtxos,
      outputUtxos: outputUtxos,
      action: "WITHDRAWAL",
      assetPubkeys: [
        new anchor.BN(0),
        hashAndTruncateToCircuit(MINT.toBytes()),
      ],
      mintPubkey: hashAndTruncateToCircuit(MINT.toBytes()),
      recipientFee: origin.publicKey,
      recipient: tokenRecipient,
      merkleTreeAssetPubkey: REGISTERED_POOL_PDA_SPL_TOKEN,
      relayerFee: new anchor.BN("10000"),
      config: { in: 2, out: 2 },
    });

    await SHIELDED_TRANSACTION.getProof();

    // await testTransaction({transaction: SHIELDED_TRANSACTION, deposit: false,provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

    try {
      let res = await SHIELDED_TRANSACTION.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await SHIELDED_TRANSACTION.checkBalances();
  });

  // doesn't work program runs out of memory
  it.skip("Withdraw 10 utxos", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY
    );
    let merkleTree = await buildMerkleTree({
      connection: provider.connection,
      config: { x: 1 }, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      poseidonHash: POSEIDON,
    });

    // get inserted leaves
    let leavesPdas = await getInsertedLeaves({
      merkleTreeProgram,
      merkleTreeIndex: mtFetched.nextIndex,
      connection: provider.connection,
    });
    let decryptedUtxo1 = await getUnspentUtxo(
      leavesPdas,
      provider,
      ENCRYPTION_KEYPAIR,
      KEYPAIR,
      FEE_ASSET,
      hashAndTruncateToCircuit(MINT.toBytes()),
      POSEIDON,
      merkleTreeProgram
    );

    const origin = new anchor.web3.Account();

    var tokenRecipient = recipientTokenAccount;

    SHIELDED_TRANSACTION = new Transaction({
      payer: ADMIN_AUTH_KEYPAIR,
      encryptionKeypair: ENCRYPTION_KEYPAIR,

      // four static config fields
      merkleTree,
      provider,
      lookupTable: LOOK_UP_TABLE,

      relayerRecipient: ADMIN_AUTH_KEYPAIR.publicKey,
      shuffleEnabled: false,
      poseidon: POSEIDON,
      verifier: new VerifierOne(),
    });

    let outputUtxos = [];

    let inputUtxos = [];
    inputUtxos.push(decryptedUtxo1);
    inputUtxos.push(new Utxo({ poseidon: POSEIDON }));
    inputUtxos.push(new Utxo({ poseidon: POSEIDON }));
    inputUtxos.push(new Utxo({ poseidon: POSEIDON }));

    await SHIELDED_TRANSACTION.compileTransaction({
      inputUtxos: inputUtxos,
      outputUtxos: outputUtxos,
      action: "WITHDRAWAL",
      assetPubkeys: [new BN(0), hashAndTruncateToCircuit(MINT.toBytes())],
      mintPubkey: hashAndTruncateToCircuit(MINT.toBytes()),
      recipientFee: origin.publicKey,
      recipient: tokenRecipient,
      merkleTreeAssetPubkey: REGISTERED_POOL_PDA_SPL_TOKEN,
      relayerFee: new anchor.BN("10000"),
      config: { in: 10, out: 2 },
    });

    await SHIELDED_TRANSACTION.getProof();

    // await testTransaction({transaction: SHIELDED_TRANSACTION, deposit: false, enabledSignerTest: false, provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

    try {
      let res = await SHIELDED_TRANSACTION.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await SHIELDED_TRANSACTION.checkBalances();
  });

});
