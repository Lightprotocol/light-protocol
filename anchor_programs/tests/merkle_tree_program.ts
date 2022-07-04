import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";
const { SystemProgram } = require('@solana/web3.js');
import { MerkleTreeProgram} from "../target/types/merkle_tree_program";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { BigNumber, providers } from 'ethers'
import { assert, expect } from "chai";
const light = require('../light-protocol-sdk');

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
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  getPdaAddresses,
  unpackLeavesAccount,
} from "./utils/unpack_accounts"
import {
  deposit,
  transact,
  executeXComputeTransactions,
  executeUpdateMerkleTreeTransactions,
  newAccountWithLamports,
  newProgramOwnedAccount,
  newAddressWithLamports,
  executeMerkleTreeUpdateTransactions
} from "./utils/test_transactions";

import {
  MERKLE_TREE_KEY,
  DEFAULT_PROGRAMS,
  ADMIN_AUTH_KEYPAIR,
  ADMIN_AUTH_KEY,
  MERKLE_TREE_SIZE,
  MERKLE_TREE_KP
  } from "./utils/constants";

  const {
    encryptionKeypair,
    relayerFee
  } = require ('./utils/testUtxos');

var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;


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

describe("merkle_tree_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();//anchor.getProvider();
  const connection = provider.connection;
  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  const attackerProgram = anchor.workspace.AttackerProgram as Program<AttackerProgram>;
  const [REGISTERED_VERIFIER_KEY] = solana.PublicKey.findProgramAddressSync(
      [verifierProgram.programId.toBuffer()],
      merkleTreeProgram.programId
    );
  const PRE_INSERTED_LEAVES_INDEX = solana.PublicKey.findProgramAddressSync(
      [MERKLE_TREE_KEY.toBuffer()],
      merkleTreeProgram.programId
    )[0];
  const MERKLE_TREE_PDA_TOKEN = solana.PublicKey.findProgramAddressSync(
      [MERKLE_TREE_KEY.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
      merkleTreeProgram.programId
    )[0];

  const AUTHORITY = solana.PublicKey.findProgramAddressSync(
      [merkleTreeProgram.programId.toBuffer()],
      verifierProgram.programId)[0];


it("Initialize Merkle Tree", async () => {
  await newAccountWithLamports(
    provider.connection,
    ADMIN_AUTH_KEYPAIR
  )
  await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
  var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
        ADMIN_AUTH_KEYPAIR.publicKey
      )

  try {
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: MERKLE_TREE_KEY,
      preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: MERKLE_TREE_KEY,
        space: MERKLE_TREE_SIZE,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: merkleTreeProgram.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
    .rpc();

  } catch(e) {
  }
  var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
        MERKLE_TREE_KEY
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
  var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
        PRE_INSERTED_LEAVES_INDEX
      )
  assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")
  UNREGISTERED_MERKLE_TREE = new anchor.web3.Account()
  UNREGISTERED_MERKLE_TREE_PDA_TOKEN = solana.PublicKey.findProgramAddressSync(
      [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
      merkleTreeProgram.programId
    )[0];

  UNREGISTERED_PRE_INSERTED_LEAVES_INDEX = solana.PublicKey.findProgramAddressSync(
      [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer()],
      merkleTreeProgram.programId
    )[0];
  try {
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
      preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
      merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: UNREGISTERED_MERKLE_TREE.publicKey,
        space: MERKLE_TREE_SIZE,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: merkleTreeProgram.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, UNREGISTERED_MERKLE_TREE])
    .rpc();
  } catch(e) {
    console.log(e)
  }
});


it("Merkle Tree update test", async () => {
    const userAccount = await newAccountWithLamports(provider.connection)
    const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

    var leavesPdas = []
    var utxos = []

    //
    // *
    // * test deposit
    // *
    //

    let merkleTree = await light.buildMerkelTree(provider.connection);

    let Keypair = new light.Keypair()

    for (var i= 0; i < 2; i++) {
      let res = await deposit({
        Keypair,
        encryptionKeypair,
        amount: 1_000_000_00,// amount
        connection: provider.connection,
        merkleTree,
        merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
        userAccount,
        verifierProgram,
        merkleTreeProgram,
        authority: AUTHORITY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider,
        relayerFee
      })
      leavesPdas.push({ isSigner: false, isWritable: true, pubkey: res[0]})
      utxos.push(res[1])
    }
    let merkleTreeWithdrawal = await light.buildMerkelTree(connection);

    const signer = await newAccountWithLamports(provider.connection)
    let merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
        [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
        merkleTreeProgram.programId)[0];
    console.log("merkleTreeUpdateState: ", merkleTreeUpdateState.toBase58())

    assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

    let merkle_tree_pubkey = MERKLE_TREE_KEY

    // Test property: 1
    // try with leavespda of higher index

    leavesPdas.reverse()
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
          new anchor.BN(0) // merkle tree index
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
            ).signers([signer]).rpc()
    }catch (e) {
      assert(e.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');
    }
    leavesPdas.reverse()
    assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

    // Test property: 1
    // try with leavespda of higher index
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
          new anchor.BN(0) // merkle tree index
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
            ).signers([signer]).rpc()
    }catch (e) {
      assert(e.error.errorCode.code == 'FirstLeavesPdaIncorrectIndex');
    }
    assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

    // Test property: 3
    // try with different Merkle tree index than leaves are queued for
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
          new anchor.BN(1) // merkle tree index
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
            ).signers([signer]).rpc()
    }catch (e) {
      assert(e.error.errorCode.code == 'ConstraintRaw');
      assert(e.error.origin == 'merkle_tree');
    }
    assert(await connection.getAccountInfo(merkleTreeUpdateState) == null)

    // insert leavesPda[0]
    await executeUpdateMerkleTreeTransactions({
      connection: provider.connection,
      signer,
      merkleTreeProgram,
      leavesPdas: [leavesPdas[0]],
      merkleTree: merkleTreeWithdrawal,
      merkle_tree_pubkey: MERKLE_TREE_KEY,
      provider
    });

    // Test property: 2
    // try to reinsert leavesPdas[0]
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
          new anchor.BN(0) // merkle tree index
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
            ).signers([signer]).rpc()
    } catch (e) {
      assert(e.error.errorCode.code == 'LeafAlreadyInserted');
    }

    // correct
    try {
      const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
          new anchor.BN(0) // merkle tree index
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
            ).signers([signer]).rpc()
    } catch (e) {
      assert(e.error.errorCode.code == 'LeafAlreadyInserted');
    }

    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      merkle_tree_pubkey,
      provider,
      merkleTreeUpdateState,
      numberOfTransactions: 10
    })
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
    merkleTree = await light.buildMerkelTree(provider.connection);


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


});
