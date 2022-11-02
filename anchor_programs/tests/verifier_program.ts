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
  executeUpdateMerkleTreeTransactions
} from "./utils/test_transactions";

import {
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  getPdaAddresses,
  unpackLeavesAccount,
} from "./utils/unpack_accounts";

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
import { MerkleTreeConfig } from './utils/merkleTree';

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

      console.log("Initing Verifier AUTHORITY");

      // try {
      //   console.log(verifierProgramZero.methods.initializeAuthority);
      //   console.log("sigbner ", ADMIN_AUTH_KEYPAIR.publicKey.toBase58());
      //
      //   const ix = await verifierProgramZero.methods.initializeAuthority().accounts({
      //     signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
      //     authority: AUTHORITY,
      //     merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
      //     ...DEFAULT_PROGRAMS
      //   })
      //   .signers([ADMIN_AUTH_KEYPAIR])
      //   .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      //   console.log("Initing Verifier AUTHORITY success");
      //
      // } catch(e) {
      //   console.log(e);
      // }
      //
      await createMint({
        authorityKeypair: ADMIN_AUTH_KEYPAIR,
        mintKeypair: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
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
    let nftMint = await createMint({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true})

    var userTokenAccount = (await newAccountWithTokens({
      connection: provider.connection,
      MINT: nftMint,
      ADMIN_AUTH_KEYPAIR,
      userAccount: new anchor.web3.Account(),
      amount: 1
    }))
  });

  async function createMint({authorityKeypair, mintKeypair = new anchor.web3.Account(),nft = false, decimals = 2}) {
    if (nft == true) {
      decimals = 0;
    }
    // await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(mintKeypair.publicKey, 1_000_000, {preflightCommitment: "confirmed", commitment: "confirmed"}));

    try {
      let space = token.MINT_SIZE

      let txCreateAccount = new solana.Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: authorityKeypair.publicKey,
          lamports: provider.connection.getMinimumBalanceForRentExemption(space),
          newAccountPubkey: mintKeypair.publicKey,
          programId: token.TOKEN_PROGRAM_ID,
          space: space

        })
      )

      let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [authorityKeypair, mintKeypair], {commitment: "finalized", preflightCommitment: 'finalized',});

      let mint = await token.createMint(
        provider.connection,
        authorityKeypair,
        authorityKeypair.publicKey,
        null, // freez auth
        decimals, //2,
        mintKeypair
      );
      // assert(MINT.toBase58() == mint.toBase58());
      console.log("mintKeypair.publicKey: ", mintKeypair.publicKey.toBase58());
      return mintKeypair.publicKey;
    } catch(e) {
      console.log(e)
    }



  }

  it.skip("Test", async () => {
    let nullifiers = []
    for (var i = 0; i < 10; i++) {
      nullifiers.push(new Uint8Array(32).fill(1))
    }
    let verifierStatePubkey = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
        verifierProgramOne.programId))[0];
    try {
      const ix1 = await verifierProgramOne.methods.shieldedTransferFirst(
        // this.proofData.proofBytes,
        new Uint8Array(32).fill(1),
        new Uint8Array(32).fill(1),
        new Uint8Array(32).fill(1),
        nullifiers,
        // [Buffer.from(this.proofData.publicInputs.nullifier0), Buffer.from(this.proofData.publicInputs.nullifier1)],
        [new Uint8Array(32).fill(1), new Uint8Array(32).fill(1)],
        new Uint8Array(32).fill(1),
        new Uint8Array(32).fill(1),
        new anchor.BN("0"),
        new anchor.BN("0"),
        Buffer.from(new Uint8Array(174).fill(1))
      ).accounts(
        {
          signingAddress:     ADMIN_AUTH_KEY,
          systemProgram:      SystemProgram.programId,
          verifierState:      verifierStatePubkey
        }
      )
      .signers([ADMIN_AUTH_KEYPAIR]).rpc({
              commitment: 'confirmed',
              preflightCommitment: 'confirmed',
            });
      console.log(ix1);

    } catch(e) {
      console.log(e);

    }

    let recentBlockhash = (await provider.connection.getRecentBlockhash()).blockhash;
  })


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

      await testTransaction(SHIELDED_TRANSACTION, true, false);


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


  it.skip("Deposit", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    for (var i = 0; i < 1; i++) {
      console.log("Deposit ", i);

      const origin = await newAccountWithLamports(provider.connection)
      const relayer = await newAccountWithLamports(provider.connection)

      let RELAYER_FEE = U64(10_000);

      let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      console.log("depositAmount: ", depositAmount);

      var userTokenAccount
      try {
        // create associated token account
        console.log("MINT ", MINT);
        console.log("ADMIN_AUTH_KEYPAIR ", ADMIN_AUTH_KEYPAIR);
        console.log("origin ", origin);
        console.log("depositAmount ", depositAmount);

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
        AUTHORITY, //delegate
        origin.publicKey, // owner
        depositAmount, //I64.readLE(1_000_000_000_00,0).toNumber(), // amount
        []
      )

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

      await testTransaction(SHIELDED_TRANSACTION);

      console.log("MINT_CIRCUIT: ", Array.from((MINT._bn.toBuffer(32).slice(0,31))));

      console.log(SHIELDED_TRANSACTION.input);

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

  async function testTransaction(SHIELDED_TRANSACTION, deposit = true, enabledSignerTest = true) {
    const origin = await newAccountWithLamports(provider.connection)

    const shieldedTxBackUp = _.cloneDeep(SHIELDED_TRANSACTION);
    console.log("SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount ", SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount);

    // Wrong pub amount
    let wrongAmount = new anchor.BN("123213").toArray()
    console.log("wrongAmount ", wrongAmount);

    SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount = Array.from([...new Array(29).fill(0), ...wrongAmount]);
    let e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log(e);

    console.log("Wrong wrongPubAmount", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)
    // Wrong feeAmount
    let wrongFeeAmount = new anchor.BN("123213").toArray()
    console.log("wrongFeeAmount ", wrongFeeAmount);

    SHIELDED_TRANSACTION.proofData.publicInputs.feeAmount = Array.from([...new Array(29).fill(0), ...wrongFeeAmount]);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong feeAmount", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    let wrongMint = new anchor.BN("123213").toArray()
    console.log("wrongMint ", wrongMint);
    console.log("SHIELDED_TRANSACTION.proofData.publicInputs ", SHIELDED_TRANSACTION.proofData.publicInputs);
    let relayer = new anchor.web3.Account();
    await createMint({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      mintKeypair: ASSET_1_ORG
    })
    SHIELDED_TRANSACTION.sender = await newAccountWithTokens({connection: provider.connection,
    MINT: ASSET_1_ORG.publicKey,
    ADMIN_AUTH_KEYPAIR,
    userAccount: relayer,
    amount: 0})
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong wrongMint", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.sender = _.cloneDeep(shieldedTxBackUp.sender);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    // Wrong encryptedOutputs
    SHIELDED_TRANSACTION.proofData.encryptedOutputs = new Uint8Array(174).fill(2);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong encryptedOutputs", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    // Wrong relayerFee
    // will result in wrong integrity hash
    SHIELDED_TRANSACTION.relayerFee = new anchor.BN("90");
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong relayerFee", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.relayerFee = _.cloneDeep(shieldedTxBackUp.relayerFee);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    for (var i in SHIELDED_TRANSACTION.proofData.publicInputs.nullifiers) {
      SHIELDED_TRANSACTION.proofData.publicInputs.nullifiers[i] = new Uint8Array(32).fill(2);
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong nullifier ", i, " ", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
      SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
      await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    }

    for (var i = 0; i < SHIELDED_TRANSACTION.proofData.publicInputs.leaves.length; i++) {
      // Wrong leafLeft
      SHIELDED_TRANSACTION.proofData.publicInputs.leaves[i] = new Uint8Array(32).fill(2);
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong leafLeft", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
      SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
    }
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    /**
    * -------- Checking Accounts -------------
    **/
    if (enabledSignerTest) {
      // Wrong signingAddress
      // will result in wrong integrity hash
      SHIELDED_TRANSACTION.relayerPubkey = origin.publicKey;
      SHIELDED_TRANSACTION.payer = origin;
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong signingAddress", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true || e.logs.includes('Program log: AnchorError caused by account: signing_address. Error Code: ConstraintAddress. Error Number: 2012. Error Message: An address constraint was violated.') == true);
      SHIELDED_TRANSACTION.relayerPubkey = _.cloneDeep(shieldedTxBackUp.relayerPubkey);
      SHIELDED_TRANSACTION.payer = _.cloneDeep(shieldedTxBackUp.payer);
      await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    }

    // Wrong recipient
    // will result in wrong integrity hash
    console.log("Wrong recipient ");

    if (deposit == true) {

      SHIELDED_TRANSACTION.recipient = origin.publicKey;
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong recipient", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
      SHIELDED_TRANSACTION.recipient = _.cloneDeep(shieldedTxBackUp.recipient);

      console.log("Wrong recipientFee ");
      // Wrong recipientFee
      // will result in wrong integrity hash
      SHIELDED_TRANSACTION.recipientFee = origin.publicKey;
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong recipientFee", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
      SHIELDED_TRANSACTION.recipientFee = _.cloneDeep(shieldedTxBackUp.recipientFee);
    } else {
      SHIELDED_TRANSACTION.sender = origin.publicKey;
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log("Wrong sender", e.logs.includes('Program log: error ProofVerificationFailed'));
      assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
      SHIELDED_TRANSACTION.sender = _.cloneDeep(shieldedTxBackUp.sender);
      await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

      console.log("Wrong senderFee ");
      // Wrong recipientFee
      // will result in wrong integrity hash
      SHIELDED_TRANSACTION.senderFee = origin.publicKey;
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log(e); // 546
      console.log("Wrong senderFee", e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:699. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.'));
      assert(e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:699. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.') == true);
      SHIELDED_TRANSACTION.senderFee = _.cloneDeep(shieldedTxBackUp.senderFee);
      await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    }

    console.log("Wrong registeredVerifierPda ");
    // Wrong registeredVerifierPda
    if (SHIELDED_TRANSACTION.registeredVerifierPda.toBase58() == REGISTERED_VERIFIER_ONE_PDA.toBase58()) {
      SHIELDED_TRANSACTION.registeredVerifierPda = REGISTERED_VERIFIER_PDA

    } else {

      SHIELDED_TRANSACTION.registeredVerifierPda = REGISTERED_VERIFIER_ONE_PDA
    }
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong registeredVerifierPda",e);
    assert(e.logs.includes('Program log: AnchorError caused by account: registered_verifier_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
    SHIELDED_TRANSACTION.registeredVerifierPda = _.cloneDeep(shieldedTxBackUp.registeredVerifierPda);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    console.log("Wrong authority ");
    // Wrong authority
    SHIELDED_TRANSACTION.signerAuthorityPubkey = new anchor.web3.Account().publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log(e);

    console.log("Wrong authority1 ", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
    assert(e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
    SHIELDED_TRANSACTION.signerAuthorityPubkey = _.cloneDeep(shieldedTxBackUp.signerAuthorityPubkey);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    // console.log("Wrong preInsertedLeavesIndex ");
    // // Wrong authority
    // SHIELDED_TRANSACTION.preInsertedLeavesIndex = SHIELDED_TRANSACTION.tokenAuthority;
    // e = await SHIELDED_TRANSACTION.sendTransaction();
    // console.log(e);
    // console.log("Wrong preInsertedLeavesIndex", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
    // assert(e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
    // SHIELDED_TRANSACTION.preInsertedLeavesIndex = _.cloneDeep(shieldedTxBackUp.preInsertedLeavesIndex);
    for (var i = 0; i < SHIELDED_TRANSACTION.nullifierPdaPubkeys.length; i++) {
      console.log("SHIELDED_TRANSACTION.nullifierPdaPubkeys.length ", SHIELDED_TRANSACTION.nullifierPdaPubkeys.length);

      // Wrong authority
      SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] = SHIELDED_TRANSACTION.nullifierPdaPubkeys[(i + 1) % (SHIELDED_TRANSACTION.nullifierPdaPubkeys.length)];
      assert(SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] != shieldedTxBackUp.nullifierPdaPubkeys[i]);
      e = await SHIELDED_TRANSACTION.sendTransaction();
      console.log(e);

      console.log("Wrong nullifierPdaPubkeys ", i," ", e.logs.includes('Program log: Passed-in pda pubkey != on-chain derived pda pubkey.'));
      assert(e.logs.includes('Program log: Passed-in pda pubkey != on-chain derived pda pubkey.') == true);
      SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] = _.cloneDeep(shieldedTxBackUp.nullifierPdaPubkeys[i]);
    }
  }
  async function  checkNfInserted(pubkeys, connection) {
    for (var i = 0; i < pubkeys.length; i++) {
      var accountInfo = await connection.getAccountInfo(
        pubkeys[i]
      )
      console.log("accountInfo ", i," ", accountInfo);

      assert(accountInfo == null);
    }

  }
  async function getUninsertedLeaves({
    merkleTreeProgram,
    merkleTreeIndex,
    connection
    // merkleTreePubkey
  }) {
    var leave_accounts: Array<{
      pubkey: PublicKey
      account: Account<Buffer>
    }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
    console.log("Total nr of accounts. ", leave_accounts.length);

    let filteredLeaves = leave_accounts
    .filter((pda) => {
      return pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber()
    }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

    return filteredLeaves.map((pda) => {
        return { isSigner: false, isWritable: false, pubkey: pda.publicKey};
    })
  }

  it.skip("Update Merkle Tree after Deposit", async () => {

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

    console.log("ENCRYPTION_KEYPAIR ", createEncryptionKeypair());
    const signer = await newAccountWithLamports(provider.connection)
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
    let merkleTreeWithdrawal = await buildMerkleTree({
        connection:provider.connection,
        config: {x:1}, // rnd filler
        merkleTreePubkey: MERKLE_TREE_KEY,
        merkleTreeProgram: merkleTreeProgram,
        poseidonHash: poseidon
      }
    );

    // await executeUpdateMerkleTreeTransactions({
    //   connection:       provider.connection,
    //   signer:           ADMIN_AUTH_KEYPAIR,
    //   merkleTreeProgram: merkleTreeProgram,
    //   leavesPdas:       leavesPdas.slice(0,5),
    //   merkleTree:       mtPrior,
    //   merkle_tree_pubkey: MERKLE_TREE_KEY,
    //   provider
    // });
  let merkleTreeUpdateState = (await solana.PublicKey.findProgramAddress(
      [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
      merkleTreeProgram.programId))[0];
  let merkle_tree_pubkey = MERKLE_TREE_KEY
  let connection = provider.connection;
  console.log("leavesPdas ", leavesPdas);
  console.log("");

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
          ]).signers([signer]).rpc()
          console.log("success 0");

  }catch (e) {
    assert(e.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');
  }
  leavesPdas.reverse()
  assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

  console.log("Test property: 1");

  // Test property: 1
  // try with leavespda of higher index
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
          ]).signers([signer]).rpc()
          console.log("success 1");

  }catch (e) {
    console.log(e);

    assert(e.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');
  }
  assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)
  console.log("Test property: 3");

  // Test property: 3
  // try with different Merkle tree index than leaves are queued for

  let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
  let different_merkle_tree = (await solana.PublicKey.findProgramAddress(
      [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(1), 8)],
      merkleTreeProgram.programId))[0];

  await merkleTreeConfig.initializeNewMerkleTree(different_merkle_tree)

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
          ]).signers([signer]).rpc()
          console.log("success 3");

  }catch (e) {
    console.log(e);

    assert(e.error.errorCode.code == 'ConstraintRaw');
    assert(e.error.origin == 'merkle_tree');
  }
  assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

  // insert leavesPda[0]
  await executeUpdateMerkleTreeTransactions({
    connection:       provider.connection,
    signer,
    merkleTreeProgram: merkleTreeProgram,
    leavesPdas:       [leavesPdas[0]],
    merkleTree:       merkleTreeWithdrawal,
    merkle_tree_pubkey: MERKLE_TREE_KEY,
    provider
  });
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
            leavesPdas
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'LeafAlreadyInserted');
  }
  console.log("initing correct update state");

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
            [leavesPdas[1]]
          ).preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
          ]).signers([signer]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'LeafAlreadyInserted');
  }
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
    leavesPdas: [leavesPdas[1]],
    current_instruction_index: 22, // 22 becaue one tx executes two instructions, it started out in ix index 1 and increments at the end of a tx
    merkleTreeProgram
  })

  // Test property: 6
  // trying to use merkleTreeUpdateState with different signer

  let maliciousSigner = await newAccountWithLamports(provider.connection)
  let maliciousMerkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
      [Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
      merkleTreeProgram.programId)[0];
  let s = false
  try {
    await executeMerkleTreeUpdateTransactions({
      signer: maliciousSigner,
      merkleTreeProgram,
      merkle_tree_pubkey,
      provider,
      merkleTreeUpdateState,
      numberOfTransactions: 1
    })
    s = true
  } catch (e) {
    assert(e.logs.indexOf('Program log: AnchorError caused by account: authority. Error Code: ConstraintAddress. Error Number: 2012. Error Message: An address constraint was violated.') != -1)
  }
  assert(s != true)
  // Test property: 4
  // try to take lock
  try {
    const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
        new anchor.BN(0) // merkle tree index
        ).accounts(
            {
              authority: maliciousSigner.publicKey,
              merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: merkle_tree_pubkey
            }
          ).remainingAccounts(
            [leavesPdas[1]]
          ).signers([maliciousSigner]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'ContractStillLocked');
  }

  // Test property: 10
  // try insert root before completing update transaction
  try {
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).remainingAccounts(
        leavesPdas
      ).signers([signer]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'MerkleTreeUpdateNotInRootInsert')
  }

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
    leavesPdas: [leavesPdas[1]],
    current_instruction_index: 56,
    merkleTreeProgram
  })

  // Test property: 9
  // final tx to insert root more leaves
  try {
    console.log("final tx to insert root more leaves")
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).remainingAccounts(
        leavesPdas.reverse()
      ).signers([signer]).rpc()
  } catch (e) {
    // console.log(e)
    assert(e.logs.indexOf('Program log: Submitted to many remaining accounts 2') != -1)
    assert(e.error.errorCode.code == 'WrongLeavesLastTx')
  }
  // reverse back
  leavesPdas.reverse()

  // Test property: 9
  // final tx to insert root different leaves
  try {
    console.log("final tx to insert root different leaves")
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).remainingAccounts(
        [leavesPdas[0]]
      ).signers([signer]).rpc()
  } catch (e) {
    // console.log(e)
    assert(e.logs.indexOf('Program log: Wrong leaf in position 0') != -1)
    assert(e.error.errorCode.code == 'WrongLeavesLastTx')
  }

  // Test property: 11
  // final tx to insert root different UNREGISTERED_MERKLE_TREE
  try {
    console.log("final tx to insert root different UNREGISTERED_MERKLE_TREE")
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: UNREGISTERED_MERKLE_TREE.publicKey
      }).remainingAccounts(
        [leavesPdas[1]]
      ).signers([signer]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'ConstraintRaw')
    assert(e.error.origin == 'merkle_tree_update_state')
  }

  // Test property: 13
  // final tx to insert root different signer
  try {
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: maliciousSigner.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).remainingAccounts(
        [leavesPdas[1]]
      ).signers([maliciousSigner]).rpc()
  } catch (e) {
    assert(e.error.errorCode.code == 'ConstraintAddress')
    assert(e.error.origin == 'authority')
  }


  var merkleTreeAccountPrior = await connection.getAccountInfo(
    merkle_tree_pubkey
  )
  merkleTree = await light.buildMerkelTree(provider.connection, MERKLE_TREE_KEY.toBytes());


  // insert correctly
  await merkleTreeProgram.methods.insertRootMerkleTree(
    new anchor.BN(254))
  .accounts({
    authority: signer.publicKey,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTree: merkle_tree_pubkey
  }).remainingAccounts(
    [leavesPdas[1]]
  ).signers([signer]).rpc()

  await checkMerkleTreeBatchUpdateSuccess({
    connection: provider.connection,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTreeAccountPrior,
    numberOfLeaves: 2,
    leavesPdas: [leavesPdas[1]],
    merkleTree: merkleTree,
    merkle_tree_pubkey: merkle_tree_pubkey
  })
})

  async function getInsertedLeaves({
    merkleTreeProgram,
    merkleTreeIndex,
    connection
    // merkleTreePubkey
  }) {
    var leave_accounts: Array<{
      pubkey: PublicKey
      account: Account<Buffer>
    }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
    console.log("Total nr of accounts. ", leave_accounts.length);

    let filteredLeaves = leave_accounts
    .filter((pda) => {
      return pda.account.leftLeafIndex.toNumber() < merkleTreeIndex.toNumber()
    }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

    return filteredLeaves;
  }

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


  it.skip("Withdraw", async () => {
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
    let decryptedUtxo1 = await getUnspentUtxo(leavesPdas);


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

    await testTransaction(SHIELDED_TRANSACTION, false);

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

  async function getUnspentUtxo(leavesPdas) {
    let decryptedUtxo1
    for (var i = 0; i < leavesPdas.length; i++) {
      console.log("iter ", i);

      // decrypt first leaves account and build utxo
      decryptedUtxo1 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(0,63))), new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(63, 87))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON)[1];
      let nullifier = decryptedUtxo1.getNullifier();

      let nullifierPubkey = (await solana.PublicKey.findProgramAddress(
          [new anchor.BN(nullifier.toString()).toBuffer(), anchor.utils.bytes.utf8.encode("nf")],
          merkleTreeProgram.programId))[0]
      let accountInfo = await provider.connection.getAccountInfo(nullifierPubkey);

      if (accountInfo == null && decryptedUtxo1.amounts[1].toString() != "0" && decryptedUtxo1.amounts[0].toString() != "0") {
        console.log("found unspent leaf");
        return decryptedUtxo1;
      } else if (i == leavesPdas.length - 1) {
        throw "no unspent leaf found";
      }

    }

  }
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
    let decryptedUtxo1 = await getUnspentUtxo(leavesPdas)
    let decryptedUtxo2


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

    await testTransaction(SHIELDED_TRANSACTION, false, false);

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
