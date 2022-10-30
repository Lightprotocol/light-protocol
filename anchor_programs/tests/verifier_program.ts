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
let ASSET_1 = new anchor.BN(new anchor.web3.Account().publicKey._bn.toBuffer(32).slice(0,31));

var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;
var UTXOS;
var MERKLE_TREE_OLD;

var MERKLE_TREE_USDC = 0
var MERKLE_TREE_PDA_TOKEN_USDC = 0
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
    let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: })
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new light.Keypair(POSEIDON, KEYPAIR.privkey)

    REGISTERED_VERIFIER_PDA = (await solana.PublicKey.findProgramAddress(
        [verifierProgramZero.programId.toBuffer()],
        merkleTreeProgram.programId
      ))[0];
    REGISTERED_VERIFIER_ONE_PDA = (await solana.PublicKey.findProgramAddress(
        [verifierProgramOne.programId.toBuffer()],
        merkleTreeProgram.programId
      ))[0];

    AUTHORITY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgramZero.programId))[0];
    AUTHORITY_ONE = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgramOne.programId))[0];

  console.log("MINT ", MINT.toBase58());


    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer(), toBufferLE(BigInt(0), 8)], // , Buffer.from(new Uint8Array(8).fill(0))
        merkleTreeProgram.programId))[0];

    PRE_INSERTED_LEAVES_INDEX = (await solana.PublicKey.findProgramAddress(
        [MERKLE_TREE_KEY.toBuffer()],
        merkleTreeProgram.programId))[0];
    POOL_TYPE_PDA = (await solana.PublicKey.findProgramAddress(
        [new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pooltype")],
        merkleTreeProgram.programId
      ))[0];
    // MERKLE_TREE_PDA_TOKEN = (await solana.PublicKey.findProgramAddress(
    //     [MERKLE_TREE_KEY.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
    //     merkleTreeProgram.programId
    //   ))[0];



    RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    TOKEN_AUTHORITY = (await solana.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("spl")],
      merkleTreeProgram.programId
    ))[0];


    REGISTERED_POOL_PDA_SPL = (await solana.PublicKey.findProgramAddress(
        [MINT.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")],
        merkleTreeProgram.programId
      ))[0];

    MERKLE_TREE_PDA_TOKEN_USDC  = (await solana.PublicKey.findProgramAddress(
      [MINT.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool")],
      merkleTreeProgram.programId
    ))[0];
    REGISTERED_POOL_PDA_SOL = (await solana.PublicKey.findProgramAddress(
      [new Uint8Array(32).fill(0), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")],
        merkleTreeProgram.programId
      ))[0];
  })

  it.skip("Initialize Merkle Tree", async () => {
    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY
    )
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);

    if (merkleTreeAccountInfoInit == null) {

      console.log("Initing MERKLE_TREE_AUTHORITY_PDA");
      MERKLE_TREE_AUTHORITY_PDA = (await solana.PublicKey.findProgramAddress(
          [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
          merkleTreeProgram.programId
        ))[0];
      try {
        const ix = await merkleTreeProgram.methods.initializeMerkleTreeAuthority().accounts({
          authority: ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
        console.log("Registering Verifier success");

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
      console.log("signer, ", signer);

      try {
        const ix = await merkleTreeProgram.methods.initializeNewMerkleTree(
        ).accounts({
          authority: ADMIN_AUTH_KEY,
          merkleTree: MERKLE_TREE_KEY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
          systemProgram: DEFAULT_PROGRAMS.systemProgram,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTreeAuthorityPda:MERKLE_TREE_AUTHORITY_PDA
        })
        .preInstructions([
          solana.ComputeBudgetProgram.requestHeapFrame({bytes: 256 * 1024}),
        ])
        .signers([ADMIN_AUTH_KEYPAIR])
        .transaction()//rpc(); // {commitment: "finalized", preflightCommitment: 'finalized',}
        let x = await solana.sendAndConfirmTransaction(
              provider.connection,
              ix,
              [ADMIN_AUTH_KEYPAIR],
              {
                commitment: 'finalized',
                preflightCommitment: 'finalized',
              },
          );

      } catch(e) {
        console.log(e);

      }



      var merkleTreeAccountInfo = await merkleTreeProgram.account.merkleTree.fetch(
        MERKLE_TREE_KEY
      )
      // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
      //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
      // )
      // if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
      //   throw "merkle tree pda size wrong after initializing";
      // }
      console.log(merkleTreeAccountInfo);

      // if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
      //   throw "merkle tree pda owner wrong after initializing";
      // }
      var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
            PRE_INSERTED_LEAVES_INDEX
          )
      assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")




      console.log("Registering Verifier");

      console.log(verifierProgramZero.programId.toBytes());

      try {
        await merkleTreeProgram.methods.registerVerifier(
          verifierProgramZero.programId
        ).accounts({
          registeredVerifierPda: REGISTERED_VERIFIER_PDA,
          authority: ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
        console.log("Registering Verifier success");

      } catch(e) {
        console.log(e);

      }

      try {
        await merkleTreeProgram.methods.registerVerifier(
          verifierProgramOne.programId
        ).accounts({
          registeredVerifierPda: REGISTERED_VERIFIER_ONE_PDA,
          authority: ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
        console.log("Registering Verifier success");

      } catch(e) {
        console.log(e);

      }

      // console.log("Initing Verifier AUTHORITY");
      //
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


      // try {
      //   let space = token.MINT_SIZE
      //
      //   let txCreateAccount = new solana.Transaction().add(
      //     SystemProgram.createAccount({
      //       fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      //       lamports: provider.connection.getMinimumBalanceForRentExemption(space),
      //       newAccountPubkey: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey,
      //       programId: token.TOKEN_PROGRAM_ID,
      //       space: space
      //
      //     })
      //   )
      //   let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [ADMIN_AUTH_KEYPAIR, solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)], {commitment: "confirmed", preflightCommitment: 'confirmed',});
      //
      //   let mint = await token.createMint(
      //     provider.connection,
      //     ADMIN_AUTH_KEYPAIR,
      //     ADMIN_AUTH_KEYPAIR.publicKey,
      //     null,
      //     2,
      //     solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
      //   );
      //   assert(MINT.toBase58() == mint.toBase58());
      //   console.log("MINT: ", MINT.toBase58());
      //
      // } catch(e) {
      //   console.log(e)
      // }
      await createMint({
        authorityKeypair: ADMIN_AUTH_KEYPAIR,
        mintKeypair: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
      })

        console.log("POOL_TYPE_PDA: ", POOL_TYPE_PDA);
      try {
        await merkleTreeProgram.methods.registerPoolType(
          Buffer.from(new Uint8Array(32).fill(0))
        ).accounts({
          registeredPoolTypePda:  POOL_TYPE_PDA,
          authority:              ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
        console.log("Registering pool_type success");

      } catch(e) {
        console.log(e);

      }

      console.log("MINT: ", MINT);


        console.log("POOL_TYPE_PDA: ", REGISTERED_POOL_PDA_SPL);


      try {
        await merkleTreeProgram.methods.registerSplPool(
        ).accounts({
          registeredAssetPoolPda:  REGISTERED_POOL_PDA_SPL,
          authority:              ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          mint: MINT,
          tokenAuthority: TOKEN_AUTHORITY,
          registeredPoolTypePda:  POOL_TYPE_PDA,
          merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN_USDC,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
        console.log("Registering spl pool success");

      } catch(e) {
        console.log(e);

      }


        console.log("REGISTERED_POOL_PDA_SOL: ", REGISTERED_POOL_PDA_SOL);

      try {
        await merkleTreeProgram.methods.registerSolPool(
        ).accounts({
          registeredAssetPoolPda:  REGISTERED_POOL_PDA_SOL,
          authority:              ADMIN_AUTH_KEY,
          merkleTreeAuthorityPda: MERKLE_TREE_AUTHORITY_PDA,
          mint: MINT,
          tokenAuthority: TOKEN_AUTHORITY,
          registeredPoolTypePda:  POOL_TYPE_PDA,
          ...DEFAULT_PROGRAMS
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
        console.log("Registering sol pool success");

      } catch(e) {
        console.log(e);

      }
    }
  });

  it("Initialize Merkle Tree Test", async () => {
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
    let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY, payer: ADMIN_AUTH_KEYPAIR})
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    let error


    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.initMerkleTreeAuthority()
      console.log("Registering AUTHORITY success");

    } catch(e) {
      console.log(e);
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
      console.log(e);
      error = e;
    }

    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined

    // initing real mt authority
    await merkleTreeConfig.initMerkleTreeAuthority()
    let newAuthority = new anchor.web3.Account();
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(newAuthority.publicKey, 1_000_000_000_000), {preflightCommitment: "confirmed", commitment: "confirmed"})

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
      console.log("Registering AUTHORITY success");
    } catch(e) {
      console.log(e);
      error =  e;
    }
    assert(error.error.errorMessage === 'InvalidAuthority');
    error = undefined
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
      console.log("Registering AUTHORITY success");
    } catch(e) {
      console.log(e);
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
      console.log("Registering Verifier success");

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
      console.log("Registering Verifier success");

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

    console.log("---------------------------------------------------------");

    await merkleTreeConfig.enableNfts(true);

    let merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority);
    assert(merkleTreeAuthority.enableNfts == true);
    await merkleTreeConfig.enableNfts(false);
    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority);
    assert(merkleTreeAuthority.enableNfts == false);


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

    console.log("---------------------------------------------------------");

    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority);
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == true);
    await merkleTreeConfig.enablePermissionlessSplTokens(false);
    merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority);
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);



    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
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
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch(e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert(error.error.errorMessage == 'The program expected this account to be already initialized');
    error = undefined

    await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));

    let registeredPoolTypePdaAccount = await merkleTreeProgram.account.registeredPoolType.fetch(merkleTreeConfig.poolTypes[0].poolPda)

    console.log(registeredPoolTypePdaAccount);
    // untested
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
    await merkleTreeConfig.enableNfts(true);
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), nftMint);

    } catch (error) {
      console.log(error);

    }
    let registeredNFTPdaAccount = await merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda)

    assert(registeredNFTPdaAccount.poolType.toString() == new Uint8Array(32).fill(0).toString())
    assert(registeredNFTPdaAccount.index.toString() == "2")
    assert(registeredNFTPdaAccount.assetPoolPubkey.toBase58() == merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].token.toBase58())

    let merkleTreeAuthority2 = await merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda)
    console.log(merkleTreeAuthority2);
    assert(merkleTreeAuthority2.registeredAssetIndex.toString() == "3");
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    await merkleTreeConfig.enablePermissionlessSplTokens(true);
    merkleTreeConfig.payer = INVALID_SIGNER;

    let mint2 = await createMint({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true})

    var userTokenAccount2 = (await newAccountWithTokens({
      connection: provider.connection,
      MINT: mint2,
      ADMIN_AUTH_KEYPAIR,
      userAccount: new anchor.web3.Account(),
      amount: 2
    }))
    await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint2);
    registeredNFTPdaAccount = await merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda)

    assert(registeredNFTPdaAccount.poolType.toString() == new Uint8Array(32).fill(0).toString())
    assert(registeredNFTPdaAccount.index.toString() == "3")
    assert(registeredNFTPdaAccount.assetPoolPubkey.toBase58() == merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].token.toBase58())


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

  it.skip("Init Address Lookup Table", async () => {
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
      MERKLE_TREE_PDA_TOKEN_USDC,
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

        merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
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
        relayerFee: U64(depositFeeAmount),
        shuffle: true,
        mintPubkey: MINT_CIRCUIT,
        sender: userTokenAccount
      });

      await SHIELDED_TRANSACTION.proof();

      await testTransaction(SHIELDED_TRANSACTION, true);

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
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

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

        merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
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
        relayerFee: U64(depositFeeAmount),
        shuffle: true,
        mintPubkey: MINT_CIRCUIT,
        sender: userTokenAccount
      });

      await SHIELDED_TRANSACTION.proof();

      await testTransaction(SHIELDED_TRANSACTION);

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

  async function testTransaction(SHIELDED_TRANSACTION, deposit = true) {
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
    SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount = _.cloneDeep(shieldedTxBackUp.proofData.publicInputs.publicAmount);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)
    // Wrong feeAmount
    let wrongFeeAmount = new anchor.BN("123213").toArray()
    console.log("wrongFeeAmount ", wrongFeeAmount);

    SHIELDED_TRANSACTION.proofData.publicInputs.feeAmount = Array.from([...new Array(29).fill(0), ...wrongFeeAmount]);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong feeAmount", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount = _.cloneDeep(shieldedTxBackUp.proofData.publicInputs.publicAmount);
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
      console.log("Wrong senderFee", e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:548. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.'));
      assert(e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:548. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.') == true);
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
    console.log("Wrong registeredVerifierPda",e.logs.includes('Program log: AnchorError caused by account: registered_verifier_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
    assert(e.logs.includes('Program log: AnchorError caused by account: registered_verifier_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
    SHIELDED_TRANSACTION.registeredVerifierPda = _.cloneDeep(shieldedTxBackUp.registeredVerifierPda);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    console.log("Wrong authority ");
    // Wrong authority
    SHIELDED_TRANSACTION.signerAuthorityPubkey = new anchor.web3.Account().publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong authority", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
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
      // console.log("accountInfo ", i," ", accountInfo);

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
        [merkleTreeProgram.programId.toBuffer()], // , Buffer.from(new Uint8Array(8).fill(0))
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
        [merkleTreeProgram.programId.toBuffer()], // , Buffer.from(new Uint8Array(8).fill(0))
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
    let decryptedUtxo1 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(0,63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(63, 87))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON);
    console.log("decryptedUtxo1: ", decryptedUtxo1);

    let decryptedUtxo2 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87,87 + 63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87 + 63, 87 + 63 + 24))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET, MINT_CIRCUIT], POSEIDON);
    console.log("decryptedUtxo2: ", decryptedUtxo2);



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

      merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
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
    inputUtxos.push(decryptedUtxo1[1])


    await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: inputUtxos,
        outputUtxos: outputUtxos,
        action: "WITHDRAWAL",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, 0],
        mintPubkey: MINT_CIRCUIT,
        // relayerFee: U64(0),//RELAYER_FEE,
        recipientFee: origin.publicKey,
        recipient: tokenRecipient
    });

    await SHIELDED_TRANSACTION.proof();

    await testTransaction(SHIELDED_TRANSACTION, false);

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
        [merkleTreeProgram.programId.toBuffer()], // , Buffer.from(new Uint8Array(8).fill(0))
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
    let decryptedUtxo1 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(0,63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(63, 87))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON);
    console.log("decryptedUtxo1: ", decryptedUtxo1);

    let decryptedUtxo2 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87,87 + 63))), new Uint8Array(Array.from(leavesPdas[0].account.encryptedUtxos.slice(87 + 63, 87 + 63 + 24))), ENCRYPTION_KEYPAIR.publicKey, ENCRYPTION_KEYPAIR, KEYPAIR, [FEE_ASSET, MINT_CIRCUIT], POSEIDON);
    console.log("decryptedUtxo2: ", decryptedUtxo2);



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

      merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
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
    inputUtxos.push(decryptedUtxo1[1])
    inputUtxos.push(new light.Utxo(POSEIDON))
    inputUtxos.push(new light.Utxo(POSEIDON))
    inputUtxos.push(new light.Utxo(POSEIDON))

    await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: inputUtxos,
        outputUtxos: outputUtxos,
        action: "WITHDRAWAL",
        assetPubkeys: [FEE_ASSET, MINT_CIRCUIT, 0],
        mintPubkey: MINT_CIRCUIT,
        // relayerFee: U64(0),//RELAYER_FEE,
        recipientFee: origin.publicKey,
        recipient: tokenRecipient
    });

    await SHIELDED_TRANSACTION.proof();

    await testTransaction(SHIELDED_TRANSACTION, false);

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
