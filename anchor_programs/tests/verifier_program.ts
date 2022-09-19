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
const token = require('@solana/spl-token')
let circomlibjs = require("circomlibjs")
import {
  shieldedTransaction,
  createEncryptionKeypair
} from "./utils/shielded_tx";
import {
  newAccountWithLamports,
  newAccountWithTokens,
  newProgramOwnedAccount
} from "./utils/test_transactions";

import {
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  getPdaAddresses,
  unpackLeavesAccount,
} from "./utils/unpack_accounts"

const {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  inputUtxoAmount,
  outputUtxoAmount,
  relayerFee,
  testInputUtxo,
  testOutputUtxo
} = require ('./utils/testUtxos');

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
  MINT
  } from "./utils/constants";


var IX_DATA;
var SIGNER;
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
import {toBufferLE} from 'bigint-buffer';
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local('http://127.0.0.1:8899', {preflightCommitment: "finalized", commitment: "finalized"});//anchor.getProvider();

  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  const attackerProgram = anchor.workspace.AttackerProgram as Program<AttackerProgram>;
  console.log(solana.PublicKey);
  // var provider

  var REGISTERED_VERIFIER_KEY;
  var PRE_INSERTED_LEAVES_INDEX;
  var MERKLE_TREE_PDA_TOKEN;
  var AUTHORITY;
  var LOOK_UP_TABLE;

  it.only("init pubkeys ", async () => {
    // provider = await anchor.getProvider('https://api.devnet.solana.com', {preflightCommitment: "confirmed", commitment: "confirmed"});
    const connection = provider.connection;
    REGISTERED_VERIFIER_KEY = (await solana.PublicKey.findProgramAddress(
        [verifierProgram.programId.toBuffer()],
        merkleTreeProgram.programId
      ))[0];
    PRE_INSERTED_LEAVES_INDEX = (await solana.PublicKey.findProgramAddress(
        [MERKLE_TREE_KEY.toBuffer()],
        merkleTreeProgram.programId
      ))[0];
    MERKLE_TREE_PDA_TOKEN = (await solana.PublicKey.findProgramAddress(
        [MERKLE_TREE_KEY.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
        merkleTreeProgram.programId
      ))[0];
    AUTHORITY = (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgram.programId))[0];
  })

  // it.only("Initialize Merkle Tree", async () => {
  //
  //     console.log("AUTHORITY: ", AUTHORITY);
  //
  //   console.log("AUTHORITY: ", Array.prototype.slice.call(AUTHORITY.toBytes()));
  //   console.log("verifierProgram.programId: ", Array.prototype.slice.call(verifierProgram.programId.toBytes()));
  //
  //   await newAccountWithLamports(
  //     provider.connection,
  //     ADMIN_AUTH_KEYPAIR
  //   )
  //   await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
  //
  //       /*
  //   // initing Lookup table
  //   //
  //   const recentSlot = (await provider.connection.getSlot()) - 10;
  //   console.log(`recentSlot ${recentSlot}`);
  //
  //   const authorityPubkey = solana.Keypair.generate().publicKey;
  //   const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
  //   const [createInstruction] = solana.AddressLookupTableProgram.createLookupTable({
  //     authority: payerPubkey,
  //     payer: payerPubkey,
  //     recentSlot,
  //   });
  //   var transaction = new solana.Transaction().add(createInstruction);
  //   const [lookupTableAddress, bumpSeed] = await solana.PublicKey.findProgramAddress(
  //     [payerPubkey.toBuffer(), toBufferLE(BigInt(recentSlot), 8)],
  //     solana.AddressLookupTableProgram.programId,
  //   );
  //   const addressesToAdd = [
  //     MERKLE_TREE_KEY,
  //     MERKLE_TREE_PDA_TOKEN,
  //     PRE_INSERTED_LEAVES_INDEX,
  //     DEFAULT_PROGRAMS.rent,
  //     DEFAULT_PROGRAMS.systemProgram,
  //     DEFAULT_PROGRAMS.tokenProgram,
  //     DEFAULT_PROGRAMS.clock
  //   ];
  //
  //   const extendInstruction = solana.AddressLookupTableProgram.extendLookupTable({
  //     lookupTable: lookupTableAddress,
  //     authority: payerPubkey,
  //     payer: payerPubkey,
  //     addresses: addressesToAdd,
  //   });
  //   transaction.add(extendInstruction);
  //   try {
  //     let res = await solana.sendAndConfirmTransaction(provider.connection, transaction, [ADMIN_AUTH_KEYPAIR]);
  //     console.log(res)
  //   } catch(e) {
  //     console.log(e);
  //     process.exit();
  //   }
  //
  //   console.log("MERKLE_TREE_KEY: ", MERKLE_TREE_KEY.toBase58())
  //   console.log("MERKLE_TREE_KEY: ", Array.prototype.slice.call(MERKLE_TREE_KEY.toBytes()))
  //   console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
  //   console.log("MERKLE_TREE_PDA_TOKEN: ", Array.prototype.slice.call(MERKLE_TREE_PDA_TOKEN.toBytes()))
  //   try {
  //     let recentBlockhash = await provider.connection.getLatestBlockhash();
  //     const ix = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
  //       authority: ADMIN_AUTH_KEY,
  //       merkleTree: MERKLE_TREE_KEY,
  //       preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
  //       merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
  //       ...DEFAULT_PROGRAMS
  //     })
  //     .preInstructions([
  //       SystemProgram.createAccount({
  //         fromPubkey: ADMIN_AUTH_KEY,
  //         newAccountPubkey: MERKLE_TREE_KEY,
  //         space: MERKLE_TREE_SIZE,
  //         lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
  //         programId: merkleTreeProgram.programId,
  //       })
  //     ])
  //     .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
  //     .transaction();
  //     ix.recentBlockhash = recentBlockhash.blockhash;
  //     ix.feePayer = ADMIN_AUTH_KEY;
  //     ix.lastValidBlockHeight = recentBlockhash.lastValidBlockHeight;
  //     console.log(ix);
  //     const compiledInstruction = ix.compileMessage();
  //     console.log("compiledInstruction0: ", ix.instructions[0])
  //     console.log("compiledInstruction1: ", ix.instructions[1])
  //     for (var i = 0; i < ix.instructions[1].keys.length; i++) {
  //       console.log(`${ix.instructions[1].keys[i].pubkey.toBase58()}, isWritable: ${ix.instructions[1].keys[i].isWritable}, isSigner: ${ix.instructions[1].keys[i].isSigner}`);
  //
  //     }
  //     console.log(`programid: ${ix.instructions[1].programId.toBase58()}`);
  //     console.log(`data: ${Array.from(ix.instructions[1].data)}`);
  //
  //     process.exit()
  //     let vTx = new solana.VersionedTransaction(
  //         new solana.MessageV0({
  //           header: {
  //             numRequiredSignatures: 1,
  //             numReadonlySignedAccounts:1,
  //             numReadonlyUnsignedAccounts:3, // might be wrong
  //           },
  //           staticAccountKeys: [ADMIN_AUTH_KEY, MERKLE_TREE_KEY, PRE_INSERTED_LEAVES_INDEX, MERKLE_TREE_PDA_TOKEN, DEFAULT_PROGRAMS.systemProgram, DEFAULT_PROGRAMS.rent],
  //           compiledInstructions: [{
  //             /// Index into the transaction keys array indicating the program account that executes this instruction /
  //             programIdIndex: 5,
  //             // Ordered indices into the transaction keys array indicating which accounts to pass to the program /
  //             accountKeyIndexes: [0, 1],
  //             // The program input data /
  //             data: "",//compiledInstruction.instructions[0].data,
  //           }],
  //           addressTableLookups: [
  //           //   {
  //           //   accountKey: lookupTableAddress,
  //           //   writableIndexes: [],
  //           //   readonlyIndexes: [0],
  //           // }
  //           // {
  //           //   accountKey: new PublicKey(3),
  //           //   writableIndexes: [1],
  //           //   readonlyIndexes: [],
  //           // }
  //         ],
  //           recentBlockhash
  //         }),
  //         [ADMIN_AUTH_KEYPAIR] //MERKLE_TREE_KP
  //     );
  //     console.log("vTx: ", vTx);
  //     let res = await solana.sendAndConfirmTransaction(provider.connection, vTx, [ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP]);
  //     console.log(res)
  //   } catch(e) {
  //     console.log("e: ", e)
  //     process.exit()
  //
  //   }*/
  //   console.log(MERKLE_TREE_KEY);
  //
  //
  //   var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
  //         MERKLE_TREE_KEY
  //       )
  //
  //   // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
  //   //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
  //   // )
  //   if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
  //     throw "merkle tree pda size wrong after initializing";
  //
  //   }
  //   if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
  //     throw "merkle tree pda owner wrong after initializing";
  //   }
  //   var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
  //         PRE_INSERTED_LEAVES_INDEX
  //       )
  //   assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")
  //   UNREGISTERED_MERKLE_TREE = new anchor.web3.Account()
  //   UNREGISTERED_MERKLE_TREE_PDA_TOKEN = await solana.PublicKey.findProgramAddress(
  //       [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
  //       merkleTreeProgram.programId
  //     )[0];
  //
  //   UNREGISTERED_PRE_INSERTED_LEAVES_INDEX = await solana.PublicKey.findProgramAddress(
  //       [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer()],
  //       merkleTreeProgram.programId
  //     )[0];
  //   try {
  //     const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
  //       authority: ADMIN_AUTH_KEY,
  //       merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
  //       preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
  //       merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
  //       ...DEFAULT_PROGRAMS
  //     })
  //     .preInstructions([
  //       SystemProgram.createAccount({
  //         fromPubkey: ADMIN_AUTH_KEY,
  //         newAccountPubkey: UNREGISTERED_MERKLE_TREE.publicKey,
  //         space: MERKLE_TREE_SIZE,
  //         lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
  //         programId: merkleTreeProgram.programId,
  //       })
  //     ])
  //     .signers([ADMIN_AUTH_KEYPAIR, UNREGISTERED_MERKLE_TREE])
  //     .rpc();
  //   } catch(e) {
  //     console.log(e)
  //   }
  //
  //
  // });

  it.only("Initialize Token Merkle tree", async () => {
    // create new token
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
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)

    MERKLE_TREE_USDC= await solana.PublicKey.createWithSeed(
        ADMIN_AUTH_KEY,
        "usdc",
        merkleTreeProgram.programId,
      )
    MERKLE_TREE_PDA_TOKEN_USDC  = (await solana.PublicKey.findProgramAddress(
          [MERKLE_TREE_USDC.toBytes(), anchor.utils.bytes.utf8.encode("merkle_tree_pda_token")],
          merkleTreeProgram.programId
        ))[0];
    PRE_INSERTED_LEAVES_INDEX_USDC = (await solana.PublicKey.findProgramAddress(
        [MERKLE_TREE_USDC.toBuffer()],
        merkleTreeProgram.programId
      ))[0];
    RENT_ESCROW = await provider.connection.getMinimumBalanceForRentExemption(256);
    RENT_VERIFIER = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    RENT_TOKEN_ACCOUNT = await provider.connection.getMinimumBalanceForRentExemption(token.ACCOUNT_SIZE)

    console.log("MERKLE_TREE_USDC: ", MERKLE_TREE_USDC.toBase58())

    console.log("MERKLE_TREE_USDC: ", Array.prototype.slice.call(MERKLE_TREE_USDC.toBytes()))
    console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", MERKLE_TREE_PDA_TOKEN_USDC.toBase58())
    console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", Array.prototype.slice.call(MERKLE_TREE_PDA_TOKEN_USDC.toBytes()))

    const signer = await newAccountWithLamports(provider.connection)

    await provider.connection.requestAirdrop(signer.publicKey, 1_000_000_000_000)
    let tokenAuthority = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("spl")],
        merkleTreeProgram.programId
      ))[0];


    try {
      const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSpl(
      ).accounts({
        authority: ADMIN_AUTH_KEYPAIR.publicKey,
        merkleTree: MERKLE_TREE_USDC,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
        merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN_USDC,
        tokenProgram:token.TOKEN_PROGRAM_ID,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        mint: MINT,
        tokenAuthority: tokenAuthority,
        rent: DEFAULT_PROGRAMS.rent
      })
      .preInstructions([
        SystemProgram.createAccountWithSeed({
          basePubkey:ADMIN_AUTH_KEY,
          seed: anchor.utils.bytes.utf8.encode("usdc"),
          fromPubkey: ADMIN_AUTH_KEY,
          newAccountPubkey: MERKLE_TREE_USDC,
          space: MERKLE_TREE_SIZE,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
          programId: merkleTreeProgram.programId,
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();
      console.log(tx);

    } catch(e) {
      console.log("e: ", e)
    }
    var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
          MERKLE_TREE_USDC
        )
    // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
    //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
    // )
    if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
      throw "merkle tree pda size wrong after initializing";

    }
    if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
      throw "merkle tree pda owner wrong after initializing";
    }

  });

  it.only("Init Address Lookup Table", async () => {
    // await newAccountWithLamports(
    //   provider.connection,
    //   ADMIN_AUTH_KEYPAIR
    // )
    await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000)

    const recentSlot = (await provider.connection.getSlot()) - 10;
    console.log("recentSlot: ", recentSlot);


    const authorityPubkey = solana.Keypair.generate().publicKey;
    const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
    const [lookupTableAddress, bumpSeed] = await solana.PublicKey.findProgramAddress(
      [payerPubkey.toBuffer(), toBufferLE(BigInt(recentSlot), 8)],
      solana.AddressLookupTableProgram.programId,
    );
    // provider.connection.opts.preflightCommitment = 'confirmed'
    // provider.connection.opts.commitment = 'confirmed'
    // console.log("here");

    console.log(provider);

    const createInstruction = solana.AddressLookupTableProgram.createLookupTable({
      authority: payerPubkey,
      payer: payerPubkey,
      recentSlot,
    })[0];

    var transaction = new solana.Transaction().add(createInstruction);
    LOOK_UP_TABLE = lookupTableAddress;
    const addressesToAdd = [
      AUTHORITY,
      SystemProgram.programId,
      merkleTreeProgram.programId,
      DEFAULT_PROGRAMS.rent,
      MERKLE_TREE_USDC,
      PRE_INSERTED_LEAVES_INDEX_USDC,
      token.TOKEN_PROGRAM_ID,
      MERKLE_TREE_PDA_TOKEN_USDC,
      MERKLE_TREE_KEY,
      MERKLE_TREE_PDA_TOKEN
    ];
    console.log("lookupTableAddress :", lookupTableAddress);
    console.log("payerPubkey :", payerPubkey);
    console.log("addressesToAdd :", addressesToAdd);

    const extendInstruction = solana.AddressLookupTableProgram.extendLookupTable({
      lookupTable: lookupTableAddress,
      authority: payerPubkey,
      payer: payerPubkey,
      addresses: addressesToAdd,
    });

    transaction.add(extendInstruction);
    let recentBlockhash = await provider.connection.getRecentBlockhash("confirmed");
    transaction.feePayer = payerPubkey;
    transaction.recentBlockhash = recentBlockhash;
    // console.log("ADMIN_AUTH_KEY: ", ADMIN_AUTH_KEYPAIR.publicKey.toBase58());

    try {
      let res = await solana.sendAndConfirmTransaction(provider.connection, transaction, [ADMIN_AUTH_KEYPAIR], {commitment: "finalized", preflightCommitment: 'finalized',});
    } catch(e) {
      console.log("e : ", e);
    }

    console.log("LOOK_UP_TABLE: ", LOOK_UP_TABLE.toBase58());
    let lookupTableAccount = await provider.connection.getAccountInfo(LOOK_UP_TABLE, "finalized");
    console.log("lookupTableAccount ", lookupTableAccount);


  });

  it("min test attackerProgram ", async () => {
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    console.log(provider);
    // LOOK_UP_TABLE = new solana.PublicKey("FiKAFMWTDfz8gNcxZeGUXyT9yAjCr7uvKFfBiVBhpUaf");
    try {
      const ix = await attackerProgram.methods.testr(
      ).accounts({
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: AUTHORITY,
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .instruction()
      console.log("LOOK_UP_TABLE: ", LOOK_UP_TABLE.toBase58());

      let recentBlockhash = (await provider.connection.getRecentBlockhash()).blockhash;
      let txMsg = new solana.TransactionMessage({payerKey: ADMIN_AUTH_KEY,instructions: [ix], recentBlockhash: recentBlockhash})
      let lookupTableAccount = await provider.connection.getAccountInfo(LOOK_UP_TABLE, "confirmed");
      console.log("lookupTableAccount ", lookupTableAccount);

      let unpackedLookupTableAccount = solana.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
      console.log("unpackedLookupTableAccount ", unpackedLookupTableAccount);
      let compiledTx = txMsg.compileToV0Message([{state: unpackedLookupTableAccount}]);
      console.log(compiledTx);

      compiledTx.addressTableLookups[0].accountKey = LOOK_UP_TABLE
      console.log(compiledTx);

      let transaction = new solana.VersionedTransaction(compiledTx);

      transaction.sign([ADMIN_AUTH_KEYPAIR])
      let serializedTx = transaction.serialize();
      console.log(provider);

      let res = await solana.sendAndConfirmRawTransaction(provider.connection,serializedTx,
        {
          commitment: 'finalized',
          preflightCommitment: 'finalized',
        });
      console.log(res);

      let tx_res = await provider.connection.getTransaction(res);
      console.log(tx_res);

    } catch(e) {
      console.log(e);

    }
  })


  it.only("Generate Proof & Deposit & Withdraw", async () => {
    const origin = await newAccountWithLamports(provider.connection)
    const relayer = await newAccountWithLamports(provider.connection)
    let ASSET = new anchor.BN(RELAYER.publicKey._bn.toString()).mod(FIELD_SIZE);
    let ASSET_1 = new anchor.BN(new anchor.web3.Account().publicKey._bn.toString()).mod(FIELD_SIZE);

    let FEE_ASSET = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE)
    let RELAYER_FEE = U64(10_000);
    let AMOUNT = 1_000_000

    let recipient_fee_account = await newProgramOwnedAccount({ connection: provider.connection,lamports: 300_000, owner: verifierProgram, account: AUTHORITY})

    let ENCRYPTION_KEYPAIR = createEncryptionKeypair()
    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let poseidon = await circomlibjs.buildPoseidonOpt();
    let KEYPAIR = new light.Keypair(poseidon);
    var userTokenAccount
    try {
      // create associated token account
      userTokenAccount = (await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: origin,
        amount: (1_000_000_000_000+10 )
      }))
      console.log("userTokenAccount ", userTokenAccount);

    } catch(e) {
      console.log(e);
    }

    await token.approve(
      provider.connection,
      origin,
      userTokenAccount,
      AUTHORITY, //delegate
      origin.publicKey, // owner
      1_000_000_000_000000, //I64.readLE(1_000_000_000_00,0).toNumber(), // amount
      []
    )
    let SHIELDED_TRANSACTION = new shieldedTransaction({
        encryptionKeypair:      ENCRYPTION_KEYPAIR,
        merkleTreePubkey:       MERKLE_TREE_USDC,
        merkleTreeAssetPubkey:  MERKLE_TREE_PDA_TOKEN_USDC,
        merkleTreeIndex:        1,
        poseidon:               poseidon,
        lookupTable:            LOOK_UP_TABLE,
        payer:                  ADMIN_AUTH_KEYPAIR,
        relayerPubkey:          ADMIN_AUTH_KEYPAIR.PublicKey,
        merkleTreeProgram,
        verifierProgram,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
        provider,
        merkleTreeFeeAssetPubkey: MERKLE_TREE_PDA_TOKEN
    });

    await SHIELDED_TRANSACTION.getMerkleTree();

    let deposit_utxo1 = new light.Utxo(poseidon,[FEE_ASSET,ASSET], [new anchor.BN(depositFeeAmount),new anchor.BN(depositAmount)], KEYPAIR)

    let outputUtxos = [deposit_utxo1];

    await SHIELDED_TRANSACTION.prepareTransactionFull({
      inputUtxos: [],
      outputUtxos,
      action: "DEPOSIT",
      assetPubkeys: [FEE_ASSET, ASSET, ASSET_1],
      relayerFee: U64(depositFeeAmount),
      shuffle: true,
      mintPubkey: ASSET,
      recipientFee: recipient_fee_account.publicKey,
      sender: userTokenAccount
    });

    let proof_data = await SHIELDED_TRANSACTION.proof();

      let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 3173760 * 3}));
      await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);
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
    })

});
