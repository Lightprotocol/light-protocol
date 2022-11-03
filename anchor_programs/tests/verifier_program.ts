import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { verifierProgramZero } from "../target/types/verifier_program_zero";
import { VerifierProgramOne } from "../target/types/verifier_program_one";
const { SystemProgram } = require('@solana/web3.js');
const nacl = require('tweetnacl');
import { MerkleTreeProgram } from "../target/types/merkle_tree_program";
import { findProgramAddress } from "@project-serum/anchor/dist/cjs/utils/pubkey";
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber, providers } from 'ethers'
const light = require('../light-protocol-sdk');
import _ from "lodash";
import { assert, expect } from "chai";
const token = require('@solana/spl-token');
let circomlibjs = require("circomlibjs");
import {toBufferLE} from 'bigint-buffer';
var _ = require('lodash');

import {
  buildMerkleTree
} from "./utils/build_merkle_tree";
import {
  shieldedTransaction,
  sendTransaction,
  sendTransaction10,
  createEncryptionKeypair
} from "./utils/shielded_tx";
import {
  newAccountWithLamports,
  newAccountWithTokens,
  newProgramOwnedAccount,
  executeUpdateMerkleTreeTransactions,
  executeMerkleTreeUpdateTransactions,
  testTransaction,
  createMint
} from "./utils/test_transactions";

import {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  inputUtxoAmount,
  outputUtxoAmount,
  relayerFee,
  testInputUtxo,
  testOutputUtxo
} from './utils/testUtxos';
import { MerkleTreeConfig, getUninsertedLeaves, getInsertedLeaves, getUnspentUtxo } from './utils/merkleTree';

import {
  checkEscrowAccountCreated,
  checkVerifierStateAccountCreated,
  checkFinalExponentiationSuccess,
  checkLastTxSuccess,
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
  checkRentExemption,
  assert_eq
} from "./utils/test_checks";


import {
  MERKLE_TREE_KEY,
  DEFAULT_PROGRAMS,
  ADMIN_AUTH_KEYPAIR,
  ADMIN_AUTH_KEY,
  MERKLE_TREE_SIZE,
  MERKLE_TREE_KP,
  MERKLE_TREE_SIGNER_AUTHORITY,
  PRIVATE_KEY,
  FIELD_SIZE,
  MINT_PRIVATE_KEY,
  MINT,
  ENCRYPTION_KEYPAIR
  } from "./utils/constants";

var MINT_CIRCUIT = new anchor.BN(MINT._bn.toBuffer(32).slice(0,31));
let FEE_ASSET = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toBuffer(32).slice(0,31))//new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE)
let ASSET_1_ORG = new anchor.web3.Account()
let ASSET_1 = new anchor.BN(ASSET_1_ORG.publicKey._bn.toBuffer(32).slice(0,31));

var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;
var UTXOS;
var MERKLE_TREE_OLD;

var MERKLE_TREE_USDC = 0
var REGISTERED_POOL_PDA_SPL_TOKEN = 0
var PRE_INSERTED_LEAVES_INDEX_USDC
var RENT_ESCROW
var RENT_VERIFIER
var RENT_TOKEN_ACCOUNT

var REGISTERED_VERIFIER_PDA;
var PRE_INSERTED_LEAVES_INDEX;
var MERKLE_TREE_PDA_TOKEN;
var AUTHORITY;
var AUTHORITY_ONE;
var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER_RECIPIENT;
var MERKLE_TREE_AUTHORITY_PDA;
var TOKEN_AUTHORITY;
var POOL_TYPE_PDA;
var REGISTERED_POOL_PDA_SPL;
var REGISTERED_POOL_PDA_SOL;
var REGISTERED_POOL_PDA;
var SHIELDED_TRANSACTION;
var REGISTERED_VERIFIER_ONE_PDA;
var INVALID_SIGNER;
var INVALID_MERKLE_TREE_AUTHORITY_PDA;
var PRIOR_UTXO;
var POOL_TYPE = new Uint8Array(32).fill(0);
var KEYPAIR :light.keypair =  {
    privkey: '0xd67b402d88fe6eb59004f4ab53b06a4b9dc72c74a05e60c31a07148eafa95896',
    pubkey: '11764594559652842781365480184568685555721424202471696567480221588056785654892',
    encryptionKey: 'qY7dymrKn4UjOe5bE4lL6jH1qfNcyX40d0plHOj2hjU='
};

const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local('http://127.0.0.1:8899', {preflightCommitment: "finalized", commitment: "finalized"});//anchor.getProvider();
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  const verifierProgramZero = anchor.workspace.VerifierProgramZero as Program<VerifierProgramZero>;
  const verifierProgramOne = anchor.workspace.VerifierProgramOne as Program<VerifierProgramOne>;

  it("init pubkeys ", async () => {
    // provider = await anchor.getProvider('https://api.devnet.solana.com', {preflightCommitment: "confirmed", commitment: "confirmed"});
    const connection = provider.connection;
    let balance = await connection.getBalance(ADMIN_AUTH_KEY, {preflightCommitment: "confirmed", commitment: "confirmed"});
    if (balance === 0) {
      await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000), {preflightCommitment: "confirmed", commitment: "confirmed"})
    }

    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)],
        merkleTreeProgram.programId))[0];

    let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
    MERKLE_TREE_AUTHORITY_PDA = await merkleTreeConfig.getMerkleTreeAuthorityPda();
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new light.Keypair(POSEIDON, KEYPAIR.privkey)
    RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;

    REGISTERED_VERIFIER_PDA =     (await merkleTreeConfig.getRegisteredVerifierPda(verifierProgramZero.programId)).registeredVerifierPda;
    REGISTERED_VERIFIER_ONE_PDA = (await merkleTreeConfig.getRegisteredVerifierPda(verifierProgramOne.programId)).registeredVerifierPda;

    AUTHORITY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgramZero.programId))[0];
    AUTHORITY_ONE = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgramOne.programId))[0];

    PRE_INSERTED_LEAVES_INDEX =  await merkleTreeConfig.getPreInsertedLeavesIndex();
    POOL_TYPE_PDA = await merkleTreeConfig.getPoolTypePda(POOL_TYPE);
    TOKEN_AUTHORITY = await merkleTreeConfig.getTokenAuthority();
    REGISTERED_POOL_PDA_SPL = (await merkleTreeConfig.getSplPoolPda(POOL_TYPE, MINT)).pda;
    REGISTERED_POOL_PDA_SPL_TOKEN   = (await merkleTreeConfig.getSplPoolPda(POOL_TYPE, MINT)).token;
    REGISTERED_POOL_PDA_SOL = (await merkleTreeConfig.getSolPoolPda(POOL_TYPE)).pda;
    console.log("MERKLE_TREE_KEY ", MERKLE_TREE_KEY.toBase58());
    console.log("REGISTERED_VERIFIER_PDA ", REGISTERED_VERIFIER_PDA.toBase58());
    console.log("REGISTERED_VERIFIER_ONE_PDA ", REGISTERED_VERIFIER_ONE_PDA.toBase58());
    console.log("AUTHORITY ", AUTHORITY.toBase58());
    console.log("AUTHORITY_ONE ", AUTHORITY_ONE.toBase58());
    console.log("PRE_INSERTED_LEAVES_INDEX ", PRE_INSERTED_LEAVES_INDEX.toBase58());
    console.log("TOKEN_AUTHORITY ", TOKEN_AUTHORITY.toBase58());
    console.log("REGISTERED_POOL_PDA_SPL ", REGISTERED_POOL_PDA_SPL.toBase58());
    console.log("REGISTERED_POOL_PDA_SPL_TOKEN ", REGISTERED_POOL_PDA_SPL_TOKEN.toBase58());
    console.log("REGISTERED_POOL_PDA_SOL ", REGISTERED_POOL_PDA_SOL.toBase58());

  })

  it.skip("test initing account after deposited sol", async () => {
    // validated not possible to reinit as long as there are funds in the address
    try {
      console.log(verifierProgramZero.methods.initializeAuthority);
      console.log("sigbner ", ADMIN_AUTH_KEYPAIR.publicKey.toBase58());

      const ix = await verifierProgramZero.methods.initializeAuthority().accounts({
        signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
        authority: AUTHORITY,
        merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
        ...DEFAULT_PROGRAMS
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      console.log("Initing Verifier AUTHORITY success");

    } catch(e) {
      console.log(e);
    }
  })

  it("Initialize Merkle Tree", async () => {
    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY
    )
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);

    if (merkleTreeAccountInfoInit == null) {
      let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })

      console.log("Initing MERKLE_TREE_AUTHORITY_PDA");

      try {
        const ix = await merkleTreeConfig.initMerkleTreeAuthority();
        console.log("initMerkleTreeAuthority success");

      } catch(e) {
        console.log(e);

      }

      console.log("AUTHORITY: ", AUTHORITY);

      console.log("AUTHORITY: ", Array.prototype.slice.call(AUTHORITY.toBytes()));
      console.log("verifierProgramZero.programId: ", Array.prototype.slice.call(verifierProgramZero.programId.toBytes()));
      console.log("MERKLE_TREE_KEY: ", MERKLE_TREE_KEY.toBase58())
      console.log("MERKLE_TREE_KEY: ", Array.prototype.slice.call(MERKLE_TREE_KEY.toBytes()))
      // console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
      // console.log("MERKLE_TREE_PDA_TOKEN: ", Array.prototype.slice.call(MERKLE_TREE_PDA_TOKEN.toBytes()))
      console.log(merkleTreeProgram.methods);
      let signer = new anchor.web3.Account();

      try {
        const ix = await merkleTreeConfig.initializeNewMerkleTree()

      } catch(e) {
        console.log(e);
      }

      console.log("Registering Verifier");
      try {
        await merkleTreeConfig.registerVerifier(verifierProgramZero.programId)
        console.log("Registering Verifier Zero success");
      } catch(e) {
        console.log(e);
      }

      try {
        await merkleTreeConfig.registerVerifier(verifierProgramOne.programId)
        console.log("Registering Verifier One success");
      } catch(e) {
        console.log(e);

      }

      await createMint({
        authorityKeypair: ADMIN_AUTH_KEYPAIR,
        mintKeypair: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY),
        provider
      })

      try {
        await merkleTreeConfig.registerPoolType(POOL_TYPE)
        console.log("Registering pool_type success");
      } catch(e) {
        console.log(e);
      }

      console.log("MINT: ", MINT);
      console.log("POOL_TYPE_PDA: ", REGISTERED_POOL_PDA_SPL);
      try {
        await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT)
        console.log("Registering spl pool success");
      } catch(e) {
        console.log(e);
      }

      console.log("REGISTERED_POOL_PDA_SOL: ", REGISTERED_POOL_PDA_SOL);
      try {
        await merkleTreeConfig.registerSolPool(POOL_TYPE)
        console.log("Registering sol pool success");
      } catch(e) {
        console.log(e);
      }
    }
  });

  it.skip("Initialize Merkle Tree Test", async () => {
    // Security Claims
    // Init authority pda
    // - can only be inited by a hardcoded pubkey
    // Update authority pda
    // - can only be invoked by current authority
    //
    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY
    )
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    INVALID_SIGNER =new anchor.web3.Account()
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(INVALID_SIGNER.publicKey, 1_000_000_000_000), {preflightCommitment: "confirmed", commitment: "confirmed"})


    MERKLE_TREE_AUTHORITY_PDA = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
        merkleTreeProgram.programId
      ))[0];

    INVALID_MERKLE_TREE_AUTHORITY_PDA = (await solana.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")],
      merkleTreeProgram.programId
    ))[0];
    let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY, payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection})
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    let error


    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.initMerkleTreeAuthority()

    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert(error.logs.includes('Program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 failed: Cross-program invocation with unauthorized signer or writable account'));
    error = undefined

    // init merkle tree with invalid signer
    try {
      await merkleTreeConfig.initMerkleTreeAuthority(INVALID_SIGNER)
      console.log("Registering AUTHORITY success");

    } catch(e) {
      error = e;
    }

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined

    // initing real mt authority
    await merkleTreeConfig.initMerkleTreeAuthority()
    await merkleTreeConfig.initializeNewMerkleTree()

    let newAuthority = new anchor.web3.Account();
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(newAuthority.publicKey, 1_000_000_000_000), {preflightCommitment: "confirmed", commitment: "confirmed"})

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey, true);
      console.log("Registering AUTHORITY success");
    } catch(e) {
      error =  e;
    }
    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey, true);
      console.log("Registering AUTHORITY success");
    } catch(e) {
      error =  e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined



    await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
    merkleTreeConfig.payer = newAuthority;
    await merkleTreeConfig.updateMerkleTreeAuthority(ADMIN_AUTH_KEYPAIR.publicKey);
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;


    // invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER
    try {
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);

    } catch(e) {
      error = e;
    }
    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid pda
    let tmp = merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda = INVALID_SIGNER.publicKey
    try {
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);

    } catch(e) {
      error = e;
    }

    assert(error.logs.includes('Program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 failed: Cross-program invocation with unauthorized signer or writable account'));
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda = tmp;
    error = undefined

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.enableNfts(true);
    } catch(e) {
      error = e;
    }
    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.enableNfts(true);
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    await merkleTreeConfig.enableNfts(true);

    let merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    assert(merkleTreeAuthority.enableNfts == true);
    await merkleTreeConfig.enableNfts(false);
    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    assert(merkleTreeAuthority.enableNfts == false);

    // update lock duration with invalid signer
    console.log("here");

    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateLockDuration(123);
    } catch(e) {
      error = e;
    }

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.updateLockDuration(123);
    } catch(e) {
      error = e;
    }

    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    await merkleTreeConfig.updateLockDuration(123);

    await merkleTreeConfig.updateLockDuration(10);


    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch(e) {
      error = e;
    }

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined


    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == true);
    await merkleTreeConfig.enablePermissionlessSplTokens(false);
    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);



    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch(e) {
      error = e;
    }

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));

    let registeredPoolTypePdaAccount = await merkleTreeProgram.account.registeredPoolType.fetch(merkleTreeConfig.poolTypes[0].poolPda)

    assert(registeredPoolTypePdaAccount.poolType.toString() == new Uint8Array(32).fill(0).toString())


    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch(e) {
      error = e;
    }
    console.log(error);

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    // valid
    await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));

    let registeredSolPdaAccount = await merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[0].pda)

    console.log(registeredSolPdaAccount);
    // untested
    assert(registeredSolPdaAccount.poolType.toString() == new Uint8Array(32).fill(0).toString())
    assert(registeredSolPdaAccount.index == 0)
    assert(registeredSolPdaAccount.assetPoolPubkey.toBase58() == merkleTreeConfig.poolPdas[0].pda.toBase58())


    let mint = await createMint({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      // mintKeypair: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
      provider
    })


    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch(e) {
      error = e;
    }
    console.log(error);

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    // valid
    await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    console.log(merkleTreeConfig.poolPdas);

    let registeredSplPdaAccount = await merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[0].pda)
    registeredSplPdaAccount = await merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda)

    console.log(registeredSplPdaAccount);
    // untested
    assert(registeredSplPdaAccount.poolType.toString() == new Uint8Array(32).fill(0).toString())
    assert(registeredSplPdaAccount.index.toString() == "1")
    assert(registeredSplPdaAccount.assetPoolPubkey.toBase58() == merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].token.toBase58())


    let merkleTreeAuthority1 = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority1);
    assert(merkleTreeAuthority1.registeredAssetIndex.toString() == "2");
    let nftMint = await createMint({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true, provider})

    var userTokenAccount = (await newAccountWithTokens({
      connection: provider.connection,
      MINT: nftMint,
      ADMIN_AUTH_KEYPAIR,
      userAccount: new anchor.web3.Account(),
      amount: 1
    }))
  });


  it("Init Address Lookup Table", async () => {
    const recentSlot = (await provider.connection.getSlot("finalized")) - 10;
    console.log("recentSlot: ", recentSlot);


    const authorityPubkey = solana.Keypair.generate().publicKey;
    const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
    const [lookupTableAddress, bumpSeed] = await solana.PublicKey.findProgramAddress(
      [payerPubkey.toBuffer(), toBufferLE(BigInt(recentSlot), 8)],
      solana.AddressLookupTableProgram.programId,
    );

    const createInstruction = solana.AddressLookupTableProgram.createLookupTable({
      authority: payerPubkey,
      payer: payerPubkey,
      recentSlot,
    })[0];
    let escrows = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("escrow")],
        verifierProgramZero.programId))[0];

    let ix0 = solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_0000});
    // let ix1 = solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: MERKLE_TREE_PDA_TOKEN, lamports: 1_000_000_0000});

    var transaction = new solana.Transaction().add(createInstruction);
    LOOK_UP_TABLE = lookupTableAddress;
    const addressesToAdd = [
      AUTHORITY,
      SystemProgram.programId,
      merkleTreeProgram.programId,
      DEFAULT_PROGRAMS.rent,
      PRE_INSERTED_LEAVES_INDEX,
      token.TOKEN_PROGRAM_ID,
      REGISTERED_POOL_PDA_SPL_TOKEN,
      MERKLE_TREE_KEY,
      escrows,
      TOKEN_AUTHORITY,
      REGISTERED_POOL_PDA_SOL
    ];
    const extendInstruction = solana.AddressLookupTableProgram.extendLookupTable({
      lookupTable: lookupTableAddress,
      authority: payerPubkey,
      payer: payerPubkey,
      addresses: addressesToAdd,
    });

    transaction.add(extendInstruction);
    transaction.add(ix0);
    // transaction.add(ix1);
    let recentBlockhash = await provider.connection.getRecentBlockhash("confirmed");
    transaction.feePayer = payerPubkey;
    transaction.recentBlockhash = recentBlockhash;

    try {
      let res = await solana.sendAndConfirmTransaction(provider.connection, transaction, [ADMIN_AUTH_KEYPAIR], {commitment: "finalized", preflightCommitment: 'finalized',});
    } catch(e) {
      console.log("e : ", e);
    }

    console.log("LOOK_UP_TABLE: ", LOOK_UP_TABLE.toBase58());
    let lookupTableAccount = await provider.connection.getAccountInfo(LOOK_UP_TABLE, "confirmed");
    assert(lookupTableAccount != null);

  });


  it.skip("Deposit 10 utxo", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

    for (var i = 0; i < 1; i++) {
      console.log("Deposit with 10 utxos ", i);

      const origin = await newAccountWithLamports(provider.connection)
      const relayer = await newAccountWithLamports(provider.connection)

      let RELAYER_FEE = U64(10_000);

      let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);


      var userTokenAccount
      console.log("MINT: ", MINT);
      console.log("ADMIN_AUTH_KEYPAIR: ", ADMIN_AUTH_KEYPAIR);
      console.log("origin: ", origin);
      console.log("depositAmount: ", depositAmount);

      try {
        // create associated token account
        userTokenAccount = (await newAccountWithTokens({
          connection: provider.connection,
          MINT,
          ADMIN_AUTH_KEYPAIR,
          userAccount: origin,
          amount: depositAmount
        }))
        console.log("userTokenAccount ", userTokenAccount.toBase58());

      } catch(e) {
        console.log(e);
      }

      await token.approve(
        provider.connection,
        origin,
        userTokenAccount,
        AUTHORITY_ONE, //delegate
        origin.publicKey, // owner
        depositAmount, //I64.readLE(1_000_000_000_00,0).toNumber(), // amount
        []
      )

      SHIELDED_TRANSACTION = new shieldedTransaction({
        // four static config fields
        lookupTable:            LOOK_UP_TABLE,
        merkleTreeFeeAssetPubkey: REGISTERED_POOL_PDA_SOL,
        merkleTreeProgram,
        verifierProgram: verifierProgramOne,

        merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN,
        merkleTreePubkey:       MERKLE_TREE_KEY,
        merkleTreeIndex:        1,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        provider,
        payer:                  ADMIN_AUTH_KEYPAIR,
        encryptionKeypair:      ENCRYPTION_KEYPAIR,
        relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,
        registeredVerifierPda:  REGISTERED_VERIFIER_ONE_PDA,
        sendTransaction: sendTransaction10
      });

      await SHIELDED_TRANSACTION.getMerkleTree();
      let inputUtxos = [new light.Utxo(POSEIDON), new light.Utxo(POSEIDON), new light.Utxo(POSEIDON), new light.Utxo(POSEIDON)];
      let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,MINT_CIRCUIT], [new anchor.BN(depositFeeAmount),new anchor.BN(depositAmount)], KEYPAIR)

      let outputUtxos = [deposit_utxo1];

      await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos,
        outputUtxos,
        action: "DEPOSIT",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, ASSET_1],
        relayerFee: U64(0),
        shuffle: true,
        mintPubkey: MINT_CIRCUIT,
        sender: userTokenAccount
      });

      await SHIELDED_TRANSACTION.proof();

      await testTransaction({SHIELDED_TRANSACTION, deposit: true, enabledSignerTest: false, provider, signer: ADMIN_AUTH_KEYPAIR, ASSET_1_ORG, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

      try {
        let res = await SHIELDED_TRANSACTION.sendTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      try {
        await SHIELDED_TRANSACTION.checkBalances()
      } catch (e) {
        console.log(e);
      }

    }

  })


  it("Deposit", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    const origin = await newAccountWithLamports(provider.connection)
    let RELAYER_FEE = U64(10_000);

    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    var userTokenAccount
    try {
      // create associated token account
      userTokenAccount = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: origin,
        amount: depositAmount * 2
      }))
      console.log("userTokenAccount ", userTokenAccount.toBase58());

    } catch(e) {
      console.log(e);
    }

    await token.approve(
      provider.connection,
      origin,
      userTokenAccount,
      AUTHORITY, //delegate
      origin.publicKey, // owner
      depositAmount * 2,
      []
    )
    for (var i = 0; i < 2; i++) {
      console.log("Deposit ", i);

      SHIELDED_TRANSACTION = new shieldedTransaction({
        // four static config fields
        lookupTable:            LOOK_UP_TABLE,
        merkleTreeFeeAssetPubkey: REGISTERED_POOL_PDA_SOL,
        merkleTreeProgram,
        verifierProgram: verifierProgramZero,

        merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN,
        merkleTreePubkey:       MERKLE_TREE_KEY,
        merkleTreeIndex:        1,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        provider,
        payer:                  ADMIN_AUTH_KEYPAIR,
        encryptionKeypair:      ENCRYPTION_KEYPAIR,
        relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,
        registeredVerifierPda:  REGISTERED_VERIFIER_PDA,
        sendTransaction
      });

      await SHIELDED_TRANSACTION.getMerkleTree();

      let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,MINT_CIRCUIT], [new anchor.BN(depositFeeAmount),new anchor.BN(depositAmount)], KEYPAIR)

      let outputUtxos = [deposit_utxo1];

      await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: [],
        outputUtxos,
        action: "DEPOSIT",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, ASSET_1],
        relayerFee: U64(0),
        shuffle: true,
        mintPubkey: new anchor.BN("123"), // input is apparently irrelevant
        sender: userTokenAccount
      });

      await SHIELDED_TRANSACTION.proof();

      await testTransaction({SHIELDED_TRANSACTION, provider, signer: ADMIN_AUTH_KEYPAIR, ASSET_1_ORG, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

      try {
        let res = await SHIELDED_TRANSACTION.sendTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      try {
        await SHIELDED_TRANSACTION.checkBalances()
      } catch (e) {
        console.log(e);
      }
    }

  })


  it("Update Merkle Tree after Deposit", async () => {

    console.log("ENCRYPTION_KEYPAIR ", createEncryptionKeypair());

    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)],
        merkleTreeProgram.programId))[0];

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(MERKLE_TREE_KEY)

    // fetch uninserted utxos from chain
    let leavesPdas = await getUninsertedLeaves({
        merkleTreeProgram,
        merkleTreeIndex: mtFetched.nextIndex,
        connection: provider.connection
    });

    let poseidon = await circomlibjs.buildPoseidonOpt();
    // build tree from chain
    let mtPrior = await buildMerkleTree({
        connection:provider.connection,
        config: {x:1}, // rnd filler
        merkleTreePubkey: MERKLE_TREE_KEY,
        merkleTreeProgram: merkleTreeProgram,
        poseidonHash: poseidon
      }
    );

    await executeUpdateMerkleTreeTransactions({
      connection:       provider.connection,
      signer:           ADMIN_AUTH_KEYPAIR,
      merkleTreeProgram: merkleTreeProgram,
      leavesPdas:       leavesPdas.slice(0,5),
      merkleTree:       mtPrior,
      merkle_tree_pubkey: MERKLE_TREE_KEY,
      provider
    });


    //check correct insert
    //  let mtOnchain = await merkleTreeProgram.account.merkleTree.all()
    // console.log("mtOnchain.roots[1] ", mtOnchain.roots[1]);
    // console.log("mtAfter.root() ", mtAfter.root());

    // assert(mtOnchain.roots[1] == mtAfter.root());
  })


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

    const signer = await newAccountWithLamports(provider.connection)
    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)],
        merkleTreeProgram.programId))[0];

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(MERKLE_TREE_KEY)
    let error

    // fetch uninserted utxos from chain
    let leavesPdas = await getUninsertedLeaves({
        merkleTreeProgram,
        merkleTreeIndex: mtFetched.nextIndex,
        connection: provider.connection
    });

    let poseidon = await circomlibjs.buildPoseidonOpt();
    // build tree from chain
    let merkleTreeWithdrawal = await buildMerkleTree({
        connection:provider.connection,
        config: {x:1}, // rnd filler
        merkleTreePubkey: MERKLE_TREE_KEY,
        merkleTreeProgram: merkleTreeProgram,
        poseidonHash: poseidon
      }
    );

  let merkleTreeUpdateState = (await solana.PublicKey.findProgramAddress(
      [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
      merkleTreeProgram.programId))[0];
  let merkle_tree_pubkey = MERKLE_TREE_KEY
  let connection = provider.connection;
  console.log("leavesPdas ", leavesPdas);
  let CONFIRMATION = {preflightCommitment: "finalized", commitment: "finalized"};
  if (leavesPdas.length > 1) {

    // test leaves with higher starting index than merkle tree next index
    leavesPdas.reverse()
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
      ).accounts(
        {
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: DEFAULT_PROGRAMS.systemProgram,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey
        }
      ).remainingAccounts(
        leavesPdas
      ).preInstructions([
        solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
      ]).signers([signer]).rpc(CONFIRMATION)
      console.log("success 0");

    }catch (e) {
      error = e
    }
    assert(error.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');

    leavesPdas.reverse()
    assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)



  console.log("Test property: 1");
  // Test property: 1
  // try with one leavespda of higher index
  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        ).accounts(
            {
              authority: signer.publicKey,
              merkleTreeUpdateState: merkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: merkle_tree_pubkey
            }
          ).remainingAccounts(
            leavesPdas[1]
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc(CONFIRMATION)
          console.log("success 1");

  } catch (e) {
    console.log(e);
    error = e
  }
  assert(error.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');

  assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)
} else {
  console.log("pdas.length <="  +   1 + " skipping some tests");
}

  // Test property: 3
  // try with different Merkle tree than leaves are queued for
  // index might be broken it is wasn't set to mut didn't update
  let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
  let different_merkle_tree = (await solana.PublicKey.findProgramAddress(
      [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(1), 8)],
      merkleTreeProgram.programId))[0];
  if (await connection.getAccountInfo(different_merkle_tree) == null) {
    await merkleTreeConfig.initializeNewMerkleTree(different_merkle_tree)
    console.log("created new merkle tree");
  }

  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        ).accounts(
            {
              authority: signer.publicKey,
              merkleTreeUpdateState: merkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: different_merkle_tree
            }
          ).remainingAccounts(
            leavesPdas
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc(CONFIRMATION)
          console.log("success 3");

  }catch (e) {
    console.log(e);
    error = e
  }
  assert(error.error.errorCode.code == 'LeavesOfWrongTree');
  assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)
  error = undefined

  // correct
  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        ).accounts(
            {
              authority: signer.publicKey,
              merkleTreeUpdateState: merkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: merkle_tree_pubkey
            }
          ).remainingAccounts(
            [leavesPdas[0]]
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc(CONFIRMATION)
  } catch (e) {
    error = e
    console.log(error);
  }
  // should not be an error
  assert(error === undefined)
  console.log("created update state ", merkleTreeUpdateState.toBase58());

  assert(await connection.getAccountInfo(merkleTreeUpdateState) != null)

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas: [leavesPdas[0]],
    current_instruction_index: 1,
    merkleTreeProgram
  })
  console.log("executeMerkleTreeUpdateTransactions 10");

  await executeMerkleTreeUpdateTransactions({
    signer,
    merkleTreeProgram,
    merkle_tree_pubkey,
    provider,
    merkleTreeUpdateState,
    numberOfTransactions: 10
  })
  console.log("checkMerkleTreeUpdateStateCreated 22");

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas: [leavesPdas[0]],
    current_instruction_index: 22, // 22 becaue one tx executes two instructions, it started out in ix index 1 and increments at the end of a tx
    merkleTreeProgram
  })

  // Test property: 6
  // trying to use merkleTreeUpdateState with different signer

  let maliciousSigner = await newAccountWithLamports(provider.connection)
  console.log("maliciousSigner: ", maliciousSigner.publicKey.toBase58());

  let maliciousMerkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
      [Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
      merkleTreeProgram.programId)[0];
  let s = false
  error = await executeMerkleTreeUpdateTransactions({
    signer: maliciousSigner,
    merkleTreeProgram,
    merkle_tree_pubkey,
    provider,
    merkleTreeUpdateState,
    numberOfTransactions: 1
  })
  console.log(error);

  assert(error.logs.includes('Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.'))

  // Test property: 4
  // try to take lock
  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        ).accounts(
            {
              authority: maliciousSigner.publicKey,
              merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: merkle_tree_pubkey
            }
          ).remainingAccounts(
            [leavesPdas[0]]
          ).signers([maliciousSigner]).rpc(CONFIRMATION)
  } catch (e) {
    error = e
    console.log(e);

  }
  assert(error.error.errorCode.code == 'ContractStillLocked');

  // Test property: 10
  // try insert root before completing update transaction
  try {
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      })
      .signers([signer]).rpc(CONFIRMATION)
  } catch (e) {
    error = e
  }
  assert(error.error.errorCode.code == 'MerkleTreeUpdateNotInRootInsert')

  // sending additional tx to finish the merkle tree update
  await executeMerkleTreeUpdateTransactions({
    signer,
    merkleTreeProgram,
    merkle_tree_pubkey,
    provider,
    merkleTreeUpdateState,
    numberOfTransactions: 50
  })

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas: [leavesPdas[0]],
    current_instruction_index: 56,
    merkleTreeProgram
  })

  // Test property: 11
  // final tx to insert root different UNREGISTERED_MERKLE_TREE
  try {
    console.log("final tx to insert root into different_merkle_tree")
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: different_merkle_tree
      })
      .signers([signer]).rpc(CONFIRMATION)
  } catch (e) {
    error = e
  }
  assert(error.error.errorCode.code == 'ContractStillLocked')

  // Test property: 13
  // final tx to insert root different signer
  try {
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: maliciousSigner.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      })
      .signers([maliciousSigner]).rpc(CONFIRMATION)
  } catch (e) {
    error = e
  }
  assert(error.error.errorCode.code == 'InvalidAuthority')


  var merkleTreeAccountPrior = await merkleTreeProgram.account.merkleTree.fetch(
    merkle_tree_pubkey
  )

  let merkleTree = await buildMerkleTree({
      connection:provider.connection,
      config: {x:1}, // rnd filler
      merkleTreePubkey: MERKLE_TREE_KEY,
      merkleTreeProgram: merkleTreeProgram,
      poseidonHash: poseidon
    }
  );

  // insert correctly
  await merkleTreeProgram.methods.insertRootMerkleTree(
    new anchor.BN(254))
  .accounts({
    authority: signer.publicKey,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTree: merkle_tree_pubkey
  })
  .signers([signer]).rpc(CONFIRMATION)
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
    merkleTreeProgram
  })


  console.log("Test property: 2");

  // Test property: 2
  // try to reinsert leavesPdas[0]
  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        ).accounts(
            {
              authority: signer.publicKey,
              merkleTreeUpdateState: merkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: merkle_tree_pubkey
            }
          ).remainingAccounts(
            [leavesPdas[0]]
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc(CONFIRMATION)
  } catch (e) {
      error = e
  }
  assert(error.error.errorCode.code == 'LeafAlreadyInserted');

})


  it.skip("Test Utxo encryption", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();
    KEYPAIR = new light.Keypair(POSEIDON)

    let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,MINT_CIRCUIT], [new anchor.BN(1),new anchor.BN(1)], KEYPAIR)
    deposit_utxo1.index = 0;
    let preCommitHash = deposit_utxo1.getCommitment();
    let preNullifier = deposit_utxo1.getNullifier();

    let nonce = nacl.randomBytes(24);
    let encUtxo = deposit_utxo1.encrypt(nonce, ENCRYPTION_KEYPAIR, ENCRYPTION_KEYPAIR);
    console.log(encUtxo);
    let decUtxo = light.Utxo.decrypt(new Uint8Array(Array.from(encUtxo.slice(0,63))),nonce, ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON)[1];
    // console.log(decUtxo);

    assert(preCommitHash == decUtxo.getCommitment(), "commitment doesnt match")
    assert(preNullifier == decUtxo.getNullifier(), "nullifier doesnt match")


  })


  it("Withdraw", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();


    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)],
        merkleTreeProgram.programId))[0];

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(MERKLE_TREE_KEY)
    let merkleTree = await buildMerkleTree({
        connection:provider.connection,
        config: {x:1}, // rnd filler
        merkleTreePubkey: MERKLE_TREE_KEY,
        merkleTreeProgram: merkleTreeProgram,
        poseidonHash: POSEIDON
      }
    );

    // get inserted leaves
    let leavesPdas = await getInsertedLeaves({
        merkleTreeProgram,
        merkleTreeIndex: mtFetched.nextIndex,
        connection: provider.connection
    });
    console.log("leavesPdas: ", leavesPdas[0].account.encryptedUtxos.toString());

    // decrypt first leaves account and build utxo
    // let decryptedUtxo1 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(0,63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(63, 87))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON);
    // console.log("decryptedUtxo1: ", decryptedUtxo1);
    //
    // let decryptedUtxo2 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87,87 + 63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87 + 63, 87 + 63 + 24))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET, MINT_CIRCUIT], POSEIDON);
    // console.log("decryptedUtxo2: ", decryptedUtxo2);
    let decryptedUtxo1 = await getUnspentUtxo(leavesPdas, provider, ENCRYPTION_KEYPAIR, KEYPAIR, FEE_ASSET,MINT_CIRCUIT, POSEIDON, merkleTreeProgram);


    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

    const origin = new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection)


    let RELAYER_FEE = U64(10_000);

    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    var userTokenAccount
    var tokenRecipient
    try {
      // create associated token account
      userTokenAccount = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: relayer,
        amount: depositAmount
      }))

      tokenRecipient = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: origin,
        amount: 0
      }))
      console.log("userTokenAccount ", userTokenAccount.toBase58());

    } catch(e) {
      console.log(e);
    }

    SHIELDED_TRANSACTION = new shieldedTransaction({
      // four static config fields
      lookupTable:            LOOK_UP_TABLE,
      merkleTreeFeeAssetPubkey: REGISTERED_POOL_PDA_SOL,
      merkleTreeProgram,
      verifierProgram: verifierProgramZero,

      merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN,
      merkleTreePubkey:       MERKLE_TREE_KEY,
      preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      provider,
      payer:                  ADMIN_AUTH_KEYPAIR,
      encryptionKeypair:      ENCRYPTION_KEYPAIR,
      relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,
      registeredVerifierPda:  REGISTERED_VERIFIER_PDA,
      merkleTree: merkleTree,
      poseidon: POSEIDON,
      sendTransaction
    });

    // let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,MINT._bn], [new anchor.BN(1),new anchor.BN(1)], KEYPAIR)

    let outputUtxos = [];

    let utxoIndex = 0;

    let inputUtxos = []
    inputUtxos.push(decryptedUtxo1)


    await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: inputUtxos,
        outputUtxos: outputUtxos,
        action: "WITHDRAWAL",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, 0],
        mintPubkey: MINT_CIRCUIT,
        recipientFee: origin.publicKey,
        recipient: tokenRecipient
    });

    await SHIELDED_TRANSACTION.proof();


    await testTransaction({SHIELDED_TRANSACTION, deposit: false,provider, signer: ADMIN_AUTH_KEYPAIR, ASSET_1_ORG, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

    console.log("MINT_CIRCUIT: ", MINT_CIRCUIT);
    console.log("MINT_CIRCUIT: ", Array.from( MINT_CIRCUIT.toBuffer(32)));
    console.log("ASSET_1: ", Array.from( ASSET_1.toBuffer(32)));

    try {
      let res = await SHIELDED_TRANSACTION.sendTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await SHIELDED_TRANSACTION.checkBalances();


  })


  it.skip("Withdraw 10 utxos", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();


    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)],
        merkleTreeProgram.programId))[0];

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(MERKLE_TREE_KEY)
    let merkleTree = await buildMerkleTree({
        connection:provider.connection,
        config: {x:1}, // rnd filler
        merkleTreePubkey: MERKLE_TREE_KEY,
        merkleTreeProgram: merkleTreeProgram,
        poseidonHash: POSEIDON
      }
    );

    // get inserted leaves
    let leavesPdas = await getInsertedLeaves({
        merkleTreeProgram,
        merkleTreeIndex: mtFetched.nextIndex,
        connection: provider.connection
    });
    let decryptedUtxo1 = await getUnspentUtxo(leavesPdas, provider, ENCRYPTION_KEYPAIR, KEYPAIR, FEE_ASSET,MINT_CIRCUIT, POSEIDON, merkleTreeProgram);


    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

    const origin = new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection)


    let RELAYER_FEE = U64(10_000);

    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    var userTokenAccount
    var tokenRecipient
    try {
      // create associated token account
      userTokenAccount = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: relayer,
        amount: depositAmount
      }))

      tokenRecipient = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: origin,
        amount: 0
      }))
      console.log("userTokenAccount ", userTokenAccount.toBase58());

    } catch(e) {
      console.log(e);
    }

    SHIELDED_TRANSACTION = new shieldedTransaction({
      // four static config fields
      lookupTable:            LOOK_UP_TABLE,
      merkleTreeFeeAssetPubkey: REGISTERED_POOL_PDA_SOL,
      merkleTreeProgram,
      verifierProgram: verifierProgramOne,

      merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN,
      merkleTreePubkey:       MERKLE_TREE_KEY,
      preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      provider,
      payer:                  ADMIN_AUTH_KEYPAIR,
      encryptionKeypair:      ENCRYPTION_KEYPAIR,
      relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,
      registeredVerifierPda:  REGISTERED_VERIFIER_ONE_PDA,
      merkleTree: merkleTree,
      poseidon: POSEIDON,
      sendTransaction: sendTransaction10
    });

    let outputUtxos = [];

    let utxoIndex = 0;

    let inputUtxos = []
    inputUtxos.push(decryptedUtxo1)
    inputUtxos.push(new light.Utxo(POSEIDON))
    inputUtxos.push(new light.Utxo(POSEIDON))
    inputUtxos.push(new light.Utxo(POSEIDON))

    await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: inputUtxos,
        outputUtxos: outputUtxos,
        action: "WITHDRAWAL",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, 0],
        mintPubkey: MINT_CIRCUIT,
        recipientFee: origin.publicKey,
        recipient: tokenRecipient
    });

    await SHIELDED_TRANSACTION.proof();


    await testTransaction({SHIELDED_TRANSACTION, deposit: false, enabledSignerTest: false, provider, signer: ADMIN_AUTH_KEYPAIR, ASSET_1_ORG, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

    try {
      let res = await SHIELDED_TRANSACTION.sendTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await SHIELDED_TRANSACTION.checkBalances();


  })


});
