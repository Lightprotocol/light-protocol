import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";
const { SystemProgram } = require('@solana/web3.js');
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
import {
  buildMerkleTree
} from "./utils/build_merkle_tree";
import {
  shieldedTransaction,
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
  console.log("e");

  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  // const attackerProgram = anchor.workspace.AttackerProgram as Program<AttackerProgram>;
  console.log(solana.PublicKey);
  // var provider

  var REGISTERED_VERIFIER_PDA;
  var PRE_INSERTED_LEAVES_INDEX;
  var MERKLE_TREE_PDA_TOKEN;
  var AUTHORITY;
  var LOOK_UP_TABLE;
  var POSEIDON;
  var RELAYER_RECIPIENT;
  var REGISTERED_VERIFIER_PDA;
  var MERKLE_TREE_AUTHORITY_PDA;
  var TOKEN_AUTHORITY;
  var POOL_TYPE_PDA;
  var REGISTERED_POOL_PDA_SPL;
  var REGISTERED_POOL_PDA_SOL;
  var REGISTERED_POOL_PDA;
  var SHIELDED_TRANSACTION;

  it("init pubkeys ", async () => {
    // provider = await anchor.getProvider('https://api.devnet.solana.com', {preflightCommitment: "confirmed", commitment: "confirmed"});
    const connection = provider.connection;
    await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000))

    REGISTERED_VERIFIER_PDA = (await solana.PublicKey.findProgramAddress(
        [verifierProgram.programId.toBuffer()],
        merkleTreeProgram.programId
      ))[0];

  console.log("MINT ", MINT.toBase58());


    MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()], // , Buffer.from(new Uint8Array(8).fill(0))
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

    AUTHORITY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgram.programId))[0];

    POSEIDON = await circomlibjs.buildPoseidonOpt();
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

  it("Initialize Merkle Tree", async () => {
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
          newAuthority: ADMIN_AUTH_KEY,
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
      console.log("verifierProgram.programId: ", Array.prototype.slice.call(verifierProgram.programId.toBytes()));
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



      // var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
      //   MERKLE_TREE_KEY
      // )
      // // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
      // //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
      // // )
      // if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
      //   throw "merkle tree pda size wrong after initializing";
      //
      // }
      // if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
      //   throw "merkle tree pda owner wrong after initializing";
      // }
      // var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
      //       PRE_INSERTED_LEAVES_INDEX
      //     )
      // assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")




      console.log("Registering Verifier");

      console.log(verifierProgram.programId.toBytes());

      try {
        await merkleTreeProgram.methods.registerVerifier(
          verifierProgram.programId
        ).accounts({
          registeredVerifierPda: REGISTERED_VERIFIER_PDA,
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

      // console.log("Initing Verifier AUTHORITY");
      //
      // try {
      //   console.log(verifierProgram.methods.initializeAuthority);
      //   console.log("sigbner ", ADMIN_AUTH_KEYPAIR.publicKey.toBase58());
      //
      //   const ix = await verifierProgram.methods.initializeAuthority().accounts({
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


      try {
        let space = token.MINT_SIZE

        let txCreateAccount = new solana.Transaction().add(
          SystemProgram.createAccount({
            fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
            lamports: provider.connection.getMinimumBalanceForRentExemption(space),
            newAccountPubkey: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey,
            programId: token.TOKEN_PROGRAM_ID,
            space: space

          })
        )
        let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [ADMIN_AUTH_KEYPAIR, solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)], {commitment: "finalized", preflightCommitment: 'finalized',});

        let mint = await token.createMint(
          provider.connection,
          ADMIN_AUTH_KEYPAIR,
          ADMIN_AUTH_KEYPAIR.publicKey,
          null,
          2,
          solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
        );
        assert(MINT.toBase58() == mint.toBase58());
        console.log("MINT: ", MINT.toBase58());

      } catch(e) {
        console.log(e)
      }


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
        .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
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
        .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
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
        .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
        console.log("Registering sol pool success");

      } catch(e) {
        console.log(e);

      }
    }
  });

  /*
  // it("Initialize Token Merkle tree", async () => {
  //   MERKLE_TREE_USDC= await solana.PublicKey.createWithSeed(
  //     ADMIN_AUTH_KEY,
  //     "usdc",
  //     merkleTreeProgram.programId,
  //   )
  //   MERKLE_TREE_PDA_TOKEN_USDC  = (await solana.PublicKey.findProgramAddress(
  //     [MERKLE_TREE_USDC.toBytes(), anchor.utils.bytes.utf8.encode("merkle_tree_pda_token")],
  //     merkleTreeProgram.programId
  //   ))[0];
  //   PRE_INSERTED_LEAVES_INDEX_USDC = (await solana.PublicKey.findProgramAddress(
  //     [MERKLE_TREE_USDC.toBuffer()],
  //     merkleTreeProgram.programId
  //   ))[0];
  //   RENT_ESCROW = await provider.connection.getMinimumBalanceForRentExemption(256);
  //   RENT_VERIFIER = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
  //   RENT_TOKEN_ACCOUNT = await provider.connection.getMinimumBalanceForRentExemption(token.ACCOUNT_SIZE)
  //
  //   var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
  //         MERKLE_TREE_USDC
  //       )
  //   if (merkleTreeAccountInfoInit == null) {
  //     // create new token
  //     try {
  //       let space = token.MINT_SIZE
  //       let txCreateAccount = new solana.Transaction().add(
  //         SystemProgram.createAccount({
  //           fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
  //           lamports: provider.connection.getMinimumBalanceForRentExemption(space),
  //           newAccountPubkey: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey,
  //           programId: token.TOKEN_PROGRAM_ID,
  //           space: space
  //
  //         })
  //       )
  //       let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [ADMIN_AUTH_KEYPAIR, solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)], {commitment: "finalized", preflightCommitment: 'finalized',});
  //
  //       let mint = await token.createMint(
  //         provider.connection,
  //         ADMIN_AUTH_KEYPAIR,
  //         ADMIN_AUTH_KEYPAIR.publicKey,
  //         null,
  //         2,
  //         solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY)
  //       );
  //       assert(MINT.toBase58() == mint.toBase58());
  //       console.log("MINT: ", MINT.toBase58());
  //
  //     } catch(e) {
  //       // console.log(e)
  //     }
  //
  //     console.log("MERKLE_TREE_USDC: ", MERKLE_TREE_USDC.toBase58())
  //
  //     console.log("MERKLE_TREE_USDC: ", Array.prototype.slice.call(MERKLE_TREE_USDC.toBytes()))
  //     console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", MERKLE_TREE_PDA_TOKEN_USDC.toBase58())
  //     console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", Array.prototype.slice.call(MERKLE_TREE_PDA_TOKEN_USDC.toBytes()))
  //
  //     const signer = await newAccountWithLamports(provider.connection)
  //
  //     await provider.connection.requestAirdrop(signer.publicKey, 1_000_000_000_000)
  //
  //
  //     try {
  //       const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSpl(
  //       ).accounts({
  //         authority: ADMIN_AUTH_KEYPAIR.publicKey,
  //         merkleTree: MERKLE_TREE_USDC,
  //         preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
  //         merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN_USDC,
  //         tokenProgram:token.TOKEN_PROGRAM_ID,
  //         systemProgram: DEFAULT_PROGRAMS.systemProgram,
  //         mint: MINT,
  //         tokenAuthority: TOKEN_AUTHORITY,
  //         rent: DEFAULT_PROGRAMS.rent
  //       })
  //       .preInstructions([
  //         SystemProgram.createAccountWithSeed({
  //           basePubkey:ADMIN_AUTH_KEY,
  //           seed: anchor.utils.bytes.utf8.encode("usdc"),
  //           fromPubkey: ADMIN_AUTH_KEY,
  //           newAccountPubkey: MERKLE_TREE_USDC,
  //           space: MERKLE_TREE_SIZE,
  //           lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
  //           programId: merkleTreeProgram.programId,
  //         })
  //       ])
  //       .signers([ADMIN_AUTH_KEYPAIR])
  //       .rpc();
  //       console.log(tx);
  //
  //     } catch(e) {
  //       // console.log("e: ", e)
  //     }
  //     var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
  //       MERKLE_TREE_USDC
  //     )
  //     // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
  //       //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
  //       // )
  //       if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
  //         throw "merkle tree pda size wrong after initializing";
  //
  //       }
  //       if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
  //         throw "merkle tree pda owner wrong after initializing";
  //       }
  //
  //   }
  //
  // });
  */

  it("Init Address Lookup Table", async () => {
    const recentSlot = (await provider.connection.getSlot()) - 10;
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
        verifierProgram.programId))[0];

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
    let lookupTableAccount = await provider.connection.getAccountInfo(LOOK_UP_TABLE, "finalized");
    assert(lookupTableAccount != null);

  });

  it("Deposit", async () => {
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

    for (var i = 0; i < 16; i++) {
      console.log("Deposit ", i);

      const origin = await newAccountWithLamports(provider.connection)
      const relayer = await newAccountWithLamports(provider.connection)
      let ASSET = new anchor.BN(new anchor.web3.Account().publicKey._bn.toString()).mod(FIELD_SIZE);
      let ASSET_1 = new anchor.BN(new anchor.web3.Account().publicKey._bn.toString()).mod(FIELD_SIZE);

      let FEE_ASSET = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE)
      let RELAYER_FEE = U64(10_000);

      let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

      let KEYPAIR = new light.Keypair(POSEIDON);
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
        verifierProgram,

        merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
        merkleTreePubkey:       MERKLE_TREE_KEY,
        merkleTreeIndex:        1,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        provider,
        payer:                  ADMIN_AUTH_KEYPAIR,
        encryptionKeypair:      ENCRYPTION_KEYPAIR,
        relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,
        registeredVerifierPda:  REGISTERED_VERIFIER_PDA
      });

      await SHIELDED_TRANSACTION.getMerkleTree();

      let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,ASSET], [new anchor.BN(depositFeeAmount),new anchor.BN(depositAmount)], KEYPAIR)

      let outputUtxos = [deposit_utxo1];

      await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: [],
        outputUtxos,
        action: "DEPOSIT",
        assetPubkeys: [FEE_ASSET, ASSET, ASSET_1],
        relayerFee: U64(depositFeeAmount),
        shuffle: true,
        mintPubkey: ASSET,
        sender: userTokenAccount
      });

      await SHIELDED_TRANSACTION.proof();

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

  it.only("Update Merkle Tree after Deposit", async () => {

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

  it.skip("Withdraw", async () => {
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
    await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);

    const origin = new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection)
    let ASSET = new anchor.BN(new anchor.web3.Account().publicKey._bn.toString()).mod(FIELD_SIZE);
    let ASSET_1 = new anchor.BN(new anchor.web3.Account().publicKey._bn.toString()).mod(FIELD_SIZE);

    let FEE_ASSET = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE)
    let RELAYER_FEE = U64(10_000);

    let ENCRYPTION_KEYPAIR = createEncryptionKeypair()
    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    let KEYPAIR = new light.Keypair(POSEIDON);
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

    await token.transfer(
      provider.connection,
      relayer,
      userTokenAccount, // from
      MERKLE_TREE_PDA_TOKEN_USDC, // to
      relayer, // owner
      depositAmount,
      [],
      {commitment: "finalized", preflightCommitment: "finalized"},
      token.TOKEN_PROGRAM_ID
    )
    console.log("PRE_INSERTED_LEAVES_INDEX: ", PRE_INSERTED_LEAVES_INDEX);

    let SHIELDED_TRANSACTION = new shieldedTransaction({
        // four static config fields
        lookupTable:            LOOK_UP_TABLE,
        merkleTreeFeeAssetPubkey: REGISTERED_POOL_PDA_SOL,
        merkleTreeProgram,
        verifierProgram,

        merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
        merkleTreePubkey:       MERKLE_TREE_KEY,
        merkleTreeIndex:        1,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        provider,
        payer:                  ADMIN_AUTH_KEYPAIR,
        encryptionKeypair:      ENCRYPTION_KEYPAIR,
        relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey
    });

    await SHIELDED_TRANSACTION.getMerkleTree();

    let deposit_utxo1 = new light.Utxo(POSEIDON,[FEE_ASSET,ASSET], [new anchor.BN(depositFeeAmount),new anchor.BN(depositAmount)], KEYPAIR)

    let outputUtxos = [deposit_utxo1];
    for (var i = 0; i<outputUtxos.length; i++) {
      SHIELDED_TRANSACTION.merkleTree.update(SHIELDED_TRANSACTION.merkleTreeLeavesIndex, outputUtxos[i].getCommitment())
      SHIELDED_TRANSACTION.merkleTreeLeavesIndex++;
    }

    let utxoIndex = 0;

    let inputUtxos = []
    inputUtxos.push(deposit_utxo1)
    // inputUtxos.push(SHIELDED_TRANSACTION.utxos[0])

    let outFeeAmount = inputUtxos[0].amounts[0]
    let withdrawalAmount = Math.floor(Math.random() * inputUtxos[0].amounts[1].toNumber());
    let outUtxoAmount = inputUtxos[0].amounts[1].sub(new anchor.BN(withdrawalAmount))
    let outUtxoAmount2 = inputUtxos[0].amounts[2]
    console.log("SHIELDED_TRANSACTION.relayerFee ", SHIELDED_TRANSACTION.relayerFee)
    outputUtxos =[
      new light.Utxo(POSEIDON,[FEE_ASSET, inputUtxos[0].assets[1], inputUtxos[0].assets[2]], [outFeeAmount.sub(new anchor.BN(SHIELDED_TRANSACTION.relayerFee.toNumber())),outUtxoAmount, outUtxoAmount2])]
    console.log("tokenRecipient: ", tokenRecipient);
    console.log("outFeeAmountPrior: ", outFeeAmount);
    console.log("outFeeAmount: ", outFeeAmount.sub(new anchor.BN(SHIELDED_TRANSACTION.relayerFee.toNumber())));
    console.log("relayerFee: ", SHIELDED_TRANSACTION.relayerFee.toNumber());


    await SHIELDED_TRANSACTION.prepareTransactionFull({
        inputUtxos: inputUtxos,
        outputUtxos: outputUtxos,
        action: "WITHDRAWAL",
        assetPubkeys: [FEE_ASSET, inputUtxos[0].assets[1], inputUtxos[0].assets[2]],
        mintPubkey: inputUtxos[0].assets[1],
        // relayerFee: U64(0),//RELAYER_FEE,
        recipientFee: origin.publicKey,
        recipient: tokenRecipient
    });

    await SHIELDED_TRANSACTION.proof();

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
