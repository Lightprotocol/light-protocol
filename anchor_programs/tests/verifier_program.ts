import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";
const { SystemProgram } = require('@solana/web3.js');

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
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';

import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";

const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber, providers } from 'ethers'
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
import _ from "lodash";

import { assert, expect } from "chai";



const MERKLE_TREE_SIGNER_AUTHORITY = new solana.PublicKey([59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42, 153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225])

const light = require('../light-protocol-sdk');



const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

import { MerkleTreeProgram, IDL } from "../target/types/merkle_tree_program";

import { publicKey, u64, u128, } from '@project-serum/borsh';
import { struct, u8, u16, u32, blob } from 'buffer-layout';

const constants:any = {};

const TYPE_PUBKEY = { array: [ 'u8', 32 ] };
const TYPE_SEED = {defined: "&[u8]"};
const TYPE_INIT_DATA = { array: [ 'u8', 642 ] };
// IDL.constants.map((item) => {
//   if(_.isEqual(item.type, TYPE_SEED)) {
//     constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
//   } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
//   {
//     constants[item.name] = JSON.parse(item.value)
//   }
// });






const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
const MERKLE_TREE_INIT_AUTHORITY = [2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
  253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
const ADMIN_AUTH_KEY = new solana.PublicKey(new Uint8Array(MERKLE_TREE_INIT_AUTHORITY));
const ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));
const MERKLE_TREE_ACC_BYTES_0 = new Uint8Array([
    242, 149, 147, 41, 62, 228, 214, 222, 231, 159, 167, 195, 10, 226, 182, 153, 84, 80, 249, 150,
    131, 112, 150, 225, 133, 131, 32, 149, 69, 188, 94, 13,
]);
const MERKLE_TREE_KP = solana.Keypair.fromSeed(MERKLE_TREE_ACC_BYTES_0);

const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;

const MERKLE_TREE_SIZE = 16658;

const MERKLE_TREE_TOKEN_ACC_BYTES_0 = new Uint8Array([
    123, 30, 128, 110, 93, 171, 2, 242, 20, 194, 175, 25, 246, 98, 182, 99, 31, 110, 119, 163, 68,
    179, 244, 89, 176, 19, 93, 136, 149, 231, 179, 213,
]);

export const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED")
export const DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};

// const PROGRAM_LAYOUT = struct([
//   u32('isInitialized'),
//   publicKey('programDataAddress'),
// ]);

var IX_DATA;
var SIGNER;
var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;

describe("verifier_program", () => {
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
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTree().accounts({
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
    console.log("e: ", e)
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
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTree().accounts({
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

/*
it("Cpi authority test", async () => {
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    console.log("ADMIN_AUTH_KEYPAIR: ", ADMIN_AUTH_KEYPAIR)
    await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
    var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
          ADMIN_AUTH_KEYPAIR.publicKey
        )
    console.log("ADMIN_AUTH_KEYPAIRAccountInfo: ", ADMIN_AUTH_KEYPAIRAccountInfo)

    console.log("MERKLE_TREE_KP: ", verifierProgram.methods);
    let nullifier0 = new Uint8Array(32).fill(2);
    let nullifier0PdaPubkey = solana.PublicKey.findProgramAddressSync(
        [Buffer.from(nullifier0), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId)[0];
    let authority = solana.PublicKey.findProgramAddressSync(
        [merkleTreeProgram.programId.toBuffer()],
        merkleTreeProgram.programId)[0];
    console.log("merkleTreeProgram.programId.toBuffer() ", Array.prototype.slice.call(merkleTreeProgram.programId.toBuffer()))
    // , anchor.utils.bytes.utf8.encode("authority")
    console.log("authority: ", Array.prototype.slice.call(authority.toBytes()))
    console.log("authority: ", authority.toBase58())


    // try calling from other program
    try {
      const tx = await attackerProgram.methods.testNullifierInsert(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.logs.indexOf('Program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL failed: Cross-program invocation with unauthorized signer or writable account') != -1)
    }

    try {
      const tx = await attackerProgram.methods.testCheckMerkleRootExists(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      })
      .preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.logs.indexOf('Program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL failed: Cross-program invocation with unauthorized signer or writable account') != -1)
    }


    try {
      const tx = await attackerProgram.methods.testInsertTwoLeaves(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.logs.indexOf('Program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL failed: Cross-program invocation with unauthorized signer or writable account') != -1)
    }

    try {
      const tx = await attackerProgram.methods.testWithdrawSol(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.logs.indexOf('Program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL failed: Cross-program invocation with unauthorized signer or writable account') != -1)
    }

    authority = solana.PublicKey.findProgramAddressSync(
        [merkleTreeProgram.programId.toBuffer()],
        attackerProgram.programId)[0];

    try {
      const tx = await attackerProgram.methods.testNullifierInsert(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.error.errorCode.code == 'ConstraintAddress')
      assert(e.error.origin == 'authority')
    }

    try {
      const tx = await attackerProgram.methods.testCheckMerkleRootExists(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      })
      .preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.error.errorCode.code == 'ConstraintAddress')
      assert(e.error.origin == 'authority')
    }


    try {
      const tx = await attackerProgram.methods.testInsertTwoLeaves(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.error.errorCode.code == 'ConstraintAddress')
      assert(e.error.origin == 'authority')
    }

    try {
      const tx = await attackerProgram.methods.testWithdrawSol(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      assert(e.error.errorCode.code == 'ConstraintAddress')
      assert(e.error.origin == 'authority')
    }

  });

  /*
  it("Register Verifier Program", async () => {
    console.log("Register Verifier Program here");
    const tx = await merkleTreeProgram.methods.registerNewId().accounts({
      authority: ADMIN_AUTH_KEY,
      registry: REGISTERED_VERIFIER_KEY,
      newId: verifierProgram.programId,
      ...DEFAULT_PROGRAMS
    })
    .signers([ADMIN_AUTH_KEYPAIR])
    .rpc();
    console.log("register new id tx: ", tx)
    const registry = await merkleTreeProgram.account.registry.fetch(REGISTERED_VERIFIER_KEY);
    console.log("registry: ", registry)
    assert(registry.id.equals(verifierProgram.programId) , 'Verifier Program Id mismatch');

  });

  it("Failed to Create AuthorityConfig for not upgradable authority", async () => {
    const programInfo = await provider.connection.getAccountInfo(merkleTreeProgram.programId);
    const programDataAddress = PROGRAM_LAYOUT.decode(programInfo.data).programDataAddress;
    const authKeypair = solana.Keypair.generate();
    await expect(
      merkleTreeProgram.methods.createAuthorityConfig().accounts({
        authority: authKeypair.publicKey,
        merkleTreeProgram: merkleTreeProgram.programId,
        authorityConfig: AUTHORITY_CONFIG_KEY,
        merkleTreeProgramData: programDataAddress,
        ...DEFAULT_PROGRAMS
      })
      .signers([authKeypair])
      .rpc()
    ).to.be.rejectedWith("0", "A raw constraint was violated");
  });
  it("Create AuthorityConfig", async () => {
    const programInfo = await provider.connection.getAccountInfo(merkleTreeProgram.programId);
    console.log(programInfo.data);
    const programDataAddress = PROGRAM_LAYOUT.decode(programInfo.data).programDataAddress;

    const tx = await merkleTreeProgram.methods.createAuthorityConfig().accounts({
      authority: (merkleTreeProgram.provider as any).wallet.pubkey,
      merkleTreeProgram: merkleTreeProgram.programId,
      authorityConfig: AUTHORITY_CONFIG_KEY,
      merkleTreeProgramData: programDataAddress,
      ...DEFAULT_PROGRAMS
    })
    .rpc();
  });
  it("Failed to update AuthorityConfig for not current authority", async () => {
    const authKeypair = solana.Keypair.generate();
    await expect(
      merkleTreeProgram.methods.updateAuthorityConfig(ADMIN_AUTH_KEY).accounts({
        authority: authKeypair.publicKey,
        authorityConfig: AUTHORITY_CONFIG_KEY,
        ...DEFAULT_PROGRAMS
      })
      .signers([authKeypair])
      .rpc()
    ).to.be.rejectedWith("0", "A raw constraint was violated");
});
  it("Update Authority Config", async () => {
    const tx = await merkleTreeProgram.methods.updateAuthorityConfig(ADMIN_AUTH_KEY).accounts({
      authority: (merkleTreeProgram.provider as any).wallet.pubkey,
      authorityConfig: AUTHORITY_CONFIG_KEY,
      ...DEFAULT_PROGRAMS
    })
    .rpc();
  });
  */

  it("Open and close escrow relayer", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()
    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(
      relayer.publicKey
    )
    var userInfoStart = await connection.getAccountInfo(
      origin.publicKey
    )
    try{
      const tx = await verifierProgram.methods.createEscrow(
            ix_data.txIntegrityHash,
            new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
            ix_data.fee,
            new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
      ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
          }
        ).signers([relayer, origin]).rpc();

    } catch (e) {
      console.log("e", e)
    }



      await checkEscrowAccountCreated({
        connection: provider.connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: relayer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });

      var relayerInfoMid = await connection.getAccountInfo(
        relayer.publicKey
      )
      var userInfoMid = await connection.getAccountInfo(
        origin.publicKey
      )
      var feeEscrowStatePubkeyInfoMid = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )

      // Third party account tries to close escrow
      const attacker = await newAccountWithLamports(provider.connection)
      try {
        await verifierProgram.methods.closeEscrow(
          ).accounts(
            {
              signingAddress: attacker.publicKey,
              verifierState: pdas.verifierStatePubkey,
              systemProgram: SystemProgram.programId,
              feeEscrowState: pdas.feeEscrowStatePubkey,
              user:           origin.publicKey,
              relayer:        attacker.publicKey,
            }
          ).signers([attacker]).rpc()

    } catch (e) {
      assert(e.error.origin == 'relayer');
      assert(e.error.errorCode.code == 'ConstraintRaw');
    }

    try {
      await verifierProgram.methods.closeEscrow(
        ).accounts(
          {
            signingAddress: attacker.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           attacker.publicKey,
            relayer:        relayer.publicKey,
          }
        ).signers([attacker]).rpc()

  } catch (e) {
    assert(e.error.origin == 'user');
    assert(e.error.errorCode.code == 'ConstraintRaw');
  }

    try {
      const tx1 = await verifierProgram.methods.closeEscrow(
      ).accounts(
        {
          signingAddress: relayer.publicKey,
          verifierState: pdas.verifierStatePubkey,
          systemProgram: SystemProgram.programId,
          feeEscrowState: pdas.feeEscrowStatePubkey,
          user:           origin.publicKey,
          relayer:        relayer.publicKey,
        }
      ).signers([relayer]).rpc();
    } catch (e) {
      console.log("e", e)
    }
    var feeEscrowStatePubkeyInfo = await connection.getAccountInfo(
      pdas.feeEscrowStatePubkey
    )
    var relayerInfoEnd = await connection.getAccountInfo(
      relayer.publicKey
    )
    var userInfoEnd = await connection.getAccountInfo(
      origin.publicKey
    )
    assert(feeEscrowStatePubkeyInfo == null, "Escrow account is not closed");
    console.log("feeEscrowStatePubkeyInfo")
    console.log("relayerInfo", relayerInfoEnd)
    console.log("userInfo", userInfoEnd)
    console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports} ${Number(relayerInfoEnd.lamports)}`)
    console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports - relayerInfoStart.lamports} ${Number(relayerInfoEnd.lamports) - relayerInfoStart.lamports}`)
    assert(relayerInfoStart.lamports == relayerInfoEnd.lamports)
    console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports} ${userInfoEnd.lamports}`)
    console.log("ix_data.extAmount: ", U64.readLE(ix_data.extAmount, 0).toString())
    console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports + U64.readLE(ix_data.extAmount, 0).toNumber()} ${Number(userInfoEnd.lamports) - userInfoStart.lamports}`)
    assert(userInfoStart.lamports == userInfoEnd.lamports)
    console.log("feeEscrowStatePubkeyInfoMid: ", feeEscrowStatePubkeyInfoMid.lamports)
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    console.log("rent: ", rent)
    console.log("escrow_amount: ", escrow_amount)
    console.log(`feeEscrowStatePubkeyInfoMid.lamports : ${feeEscrowStatePubkeyInfoMid.lamports} ${escrow_amount + rent} `)
    assert(feeEscrowStatePubkeyInfoMid.lamports == escrow_amount + rent)
    assert(userInfoStart.lamports == userInfoMid.lamports + escrow_amount)


  })

  it("Open and close escrow user", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()
    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(
      relayer.publicKey
    )
    var userInfoStart = await connection.getAccountInfo(
      origin.publicKey
    )
    try{
      const tx = await verifierProgram.methods.createEscrow(
            ix_data.txIntegrityHash,
            new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
            ix_data.fee,
            new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
      ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
          }
        ).signers([relayer, origin]).rpc();

    } catch (e) {
      console.log("e", e)
    }



      await checkEscrowAccountCreated({
        connection: provider.connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: relayer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });

      var relayerInfoMid = await connection.getAccountInfo(
        relayer.publicKey
      )
      var userInfoMid = await connection.getAccountInfo(
        origin.publicKey
      )
      var feeEscrowStatePubkeyInfoMid = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )

      // Third party account tries to close escrow
      const attacker = await newAccountWithLamports(provider.connection)
      try {
        await verifierProgram.methods.closeEscrow(
          ).accounts(
            {
              signingAddress: attacker.publicKey,
              verifierState: pdas.verifierStatePubkey,
              systemProgram: SystemProgram.programId,
              feeEscrowState: pdas.feeEscrowStatePubkey,
              user:           origin.publicKey,
              relayer:        attacker.publicKey,
            }
          ).signers([attacker]).rpc()

    } catch (e) {
      assert(e.error.origin == 'relayer');
      assert(e.error.errorCode.code == 'ConstraintRaw');
    }

    try {
      await verifierProgram.methods.closeEscrow(
        ).accounts(
          {
            signingAddress: attacker.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           attacker.publicKey,
            relayer:        relayer.publicKey,
          }
        ).signers([attacker]).rpc()

  } catch (e) {
    assert(e.error.origin == 'user');
    assert(e.error.errorCode.code == 'ConstraintRaw');
  }

    try {
      const tx1 = await verifierProgram.methods.closeEscrow(
      ).accounts(
        {
          signingAddress: origin.publicKey,
          verifierState: pdas.verifierStatePubkey,
          systemProgram: SystemProgram.programId,
          feeEscrowState: pdas.feeEscrowStatePubkey,
          user:           origin.publicKey,
          relayer:        relayer.publicKey,
        }
      ).signers([origin]).rpc();
    } catch (e) {
      console.log("e", e)
    }
    var feeEscrowStatePubkeyInfo = await connection.getAccountInfo(
      pdas.feeEscrowStatePubkey
    )
    var relayerInfoEnd = await connection.getAccountInfo(relayer.publicKey)
    var userInfoEnd = await connection.getAccountInfo(origin.publicKey)

    assert(feeEscrowStatePubkeyInfo == null, "Escrow account is not closed");
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    assert(userInfoStart.lamports == userInfoEnd.lamports)
    assert(relayerInfoStart.lamports == relayerInfoEnd.lamports)
    assert(feeEscrowStatePubkeyInfoMid.lamports == escrow_amount + rent)
    assert(userInfoStart.lamports == userInfoMid.lamports + escrow_amount)
  })

  it("Open and close escrow after 10 tx", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()
    let merkle_tree_pubkey = MERKLE_TREE_KEY
    // let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    let rent_verifier = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    console.log("provider.wallet: ", provider.wallet)
    provider.wallet.payer = relayer
    let nr_tx = 10;
    let tx_cost = (nr_tx + 1) * 5000
    // const burnerUserAccount = await newAccountWithLamports(connection)
    let merkleTree = await light.buildMerkelTree(provider.connection);

    let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)
    let deposit_utxo2 = new light.Utxo(BigNumber.from(amount), Keypair)

    let inputUtxos = [new light.Utxo(), new light.Utxo()]
    let outputUtxos = [deposit_utxo1, deposit_utxo2 ]

    const data = await light.getProof(
      inputUtxos,
      outputUtxos,
      merkleTree,
      deposit_utxo1.amount.add(deposit_utxo2.amount),
      U64(0),
      MERKLE_TREE_PDA_TOKEN.toBase58(),
      relayer.publicKey.toBase58(),
      'DEPOSIT',
      encryptionKeypair
    )
    let ix_data = parse_instruction_data_bytes(data);

    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(relayer.publicKey)
    var userInfoStart = await connection.getAccountInfo(origin.publicKey)

    try{
      const tx = await verifierProgram.methods.createEscrow(
            ix_data.txIntegrityHash,
            new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
            ix_data.fee,
            new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
      ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
          }
        ).signers([relayer, origin]).rpc();
    } catch (e) {
      console.log("e", e)
    }



      await checkEscrowAccountCreated({
        connection: provider.connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: relayer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });

      var relayerInfoMid = await connection.getAccountInfo(
        relayer.publicKey
      )
      console.log(relayerInfoMid.lamports, relayerInfoStart.lamports - rent - rent_verifier)
      assert(relayerInfoMid.lamports == relayerInfoStart.lamports - rent - rent_verifier)
      var userInfoMid = await connection.getAccountInfo(
        origin.publicKey
      )
      var feeEscrowStatePubkeyInfoMid = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )

      try  {
        const tx = await verifierProgram.methods.createVerifierState(
              ix_data.proofAbc,
              ix_data.rootHash,
              ix_data.amount,
              ix_data.txIntegrityHash,
              ix_data.nullifier0,
              ix_data.nullifier1,
              ix_data.leafRight,
              ix_data.leafLeft,
              ix_data.recipient,
              ix_data.extAmount,
              ix_data.relayer,
              ix_data.fee,
              ix_data.encryptedUtxos,
              ix_data.merkleTreeIndex
              ).accounts(
                  {
                    signingAddress: relayer.publicKey,
                    verifierState: pdas.verifierStatePubkey,
                    systemProgram: SystemProgram.programId,
                    merkleTree: merkle_tree_pubkey,
                    programMerkleTree:  merkleTreeProgram.programId,
                  }
              ).signers([relayer]).transaction()
            await provider.sendAndConfirm(tx, [relayer])
      } catch(e) {
        console.log(e)
        process.exit()
      }

      checkVerifierStateAccountCreated({
        connection:connection,
        pda: pdas.verifierStatePubkey,
        ix_data,
        relayer_pubkey:relayer.publicKey
      })

      await executeXComputeTransactions({
        number_of_transactions: nr_tx,
        signer: relayer,
        pdas: pdas,
        program: verifierProgram,
        provider:provider
      })
      var relayerInfoMid2 = await connection.getAccountInfo(
        relayer.publicKey
      )
      console.log(`relayerInfoMid ${relayerInfoMid.lamports - tx_cost} relayerInfoMid2 ${relayerInfoMid2.lamports}`)
      assert(relayerInfoMid.lamports - tx_cost == relayerInfoMid2.lamports)

    try {
      const txUserClose = await verifierProgram.methods.closeEscrow(
      ).accounts(
        {
          signingAddress: origin.publicKey,
          verifierState: pdas.verifierStatePubkey,
          systemProgram: SystemProgram.programId,
          feeEscrowState: pdas.feeEscrowStatePubkey,
          user:           origin.publicKey,
          relayer:        relayer.publicKey,
        }
      ).signers([origin]).transaction();
      await provider.sendAndConfirm(txUserClose, [origin])

    } catch (e) {
      assert(e.logs[2] == 'Program log: AnchorError thrown in programs/verifier_program/src/escrow/close_escrow_state.rs:45. Error Code: NotTimedOut. Error Number: 6006. Error Message: Closing escrow state failed relayer not timed out..');
    }

    try {
      const tx1relayer = await verifierProgram.methods.closeEscrow(
      ).accounts(
        {
          signingAddress: relayer.publicKey,
          verifierState: pdas.verifierStatePubkey,
          systemProgram: SystemProgram.programId,
          feeEscrowState: pdas.feeEscrowStatePubkey,
          user:           origin.publicKey,
          relayer:        relayer.publicKey,
        }
      ).signers([relayer]).transaction();
      await provider.sendAndConfirm(tx1relayer, [relayer])

    } catch (e) {
      console.log("etx1relayer", e)
    }
    var feeEscrowStatePubkeyInfo = await connection.getAccountInfo(
      pdas.feeEscrowStatePubkey
    )
    var relayerInfoEnd = await connection.getAccountInfo(
      relayer.publicKey
    )
    var userInfoEnd = await connection.getAccountInfo(
      origin.publicKey
    )
    assert(feeEscrowStatePubkeyInfo == null, "Escrow account is not closed");
    console.log("feeEscrowStatePubkeyInfo")
    console.log("relayerInfoEnd", relayerInfoEnd)
    console.log("userInfoEnd", userInfoEnd)
    console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports} ${Number(relayerInfoEnd.lamports)}`)
    console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports - relayerInfoStart.lamports} ${Number(relayerInfoEnd.lamports) - relayerInfoStart.lamports}`)
    assert(relayerInfoStart.lamports - 5000 == Number(relayerInfoEnd.lamports))

    console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports} ${userInfoEnd.lamports}`)
    console.log("ix_data.extAmount: ", U64.readLE(ix_data.extAmount, 0).toString())
    console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports + U64.readLE(ix_data.extAmount, 0).toNumber()} ${Number(userInfoEnd.lamports) - userInfoStart.lamports}`)

    console.log("feeEscrowStatePubkeyInfoMid: ", feeEscrowStatePubkeyInfoMid.lamports)
    console.log("rent: ", rent)
    console.log("escrow_amount: ", escrow_amount)
    console.log(`feeEscrowStatePubkeyInfoMid.lamports : ${feeEscrowStatePubkeyInfoMid.lamports} ${escrow_amount + rent} `)
    assert(userInfoStart.lamports - tx_cost == userInfoEnd.lamports)
    assert(feeEscrowStatePubkeyInfoMid.lamports == escrow_amount + rent)
    assert(userInfoStart.lamports == userInfoMid.lamports + escrow_amount)

  })

  it("reinit verifier state after 10 tx", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()
    let merkle_tree_pubkey = MERKLE_TREE_KEY
    // let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    let rent_verifier = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    provider.wallet.payer = relayer
    let nr_tx = 10;
    let tx_cost = (nr_tx + 1) * 5000
    // const burnerUserAccount = await newAccountWithLamports(connection)
    let merkleTree = await light.buildMerkelTree(provider.connection);

    let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)
    let deposit_utxo2 = new light.Utxo(BigNumber.from(amount), Keypair)

    let inputUtxos = [new light.Utxo(), new light.Utxo()]
    let outputUtxos = [deposit_utxo1, deposit_utxo2 ]

    const data = await light.getProof(
      inputUtxos,
      outputUtxos,
      merkleTree,
      deposit_utxo1.amount.add(deposit_utxo2.amount),
      U64(0),
      MERKLE_TREE_PDA_TOKEN.toBase58(),
      relayer.publicKey.toBase58(),
      'DEPOSIT',
      encryptionKeypair
    )
    let ix_data = parse_instruction_data_bytes(data);
    IX_DATA = ix_data
    SIGNER = relayer
    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(relayer.publicKey)
    var userInfoStart = await connection.getAccountInfo(origin.publicKey)

    try{
      const tx = await verifierProgram.methods.createEscrow(
            ix_data.txIntegrityHash,
            new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
            ix_data.fee,
            new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
      ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
          }
        ).signers([relayer, origin]).rpc();
    } catch (e) {
      console.log("e", e)
    }



      await checkEscrowAccountCreated({
        connection: provider.connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: relayer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });

      var relayerInfoMid = await connection.getAccountInfo(
        relayer.publicKey
      )
      console.log(relayerInfoMid.lamports, relayerInfoStart.lamports - rent - rent_verifier)
      assert(relayerInfoMid.lamports == relayerInfoStart.lamports - rent - rent_verifier)
      var userInfoMid = await connection.getAccountInfo(
        origin.publicKey
      )
      var feeEscrowStatePubkeyInfoMid = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )

      try  {
        const tx = await verifierProgram.methods.createVerifierState(
              ix_data.proofAbc,
              ix_data.rootHash,
              ix_data.amount,
              ix_data.txIntegrityHash,
              ix_data.nullifier0,
              ix_data.nullifier1,
              ix_data.leafRight,
              ix_data.leafLeft,
              ix_data.recipient,
              ix_data.extAmount,
              ix_data.relayer,
              ix_data.fee,
              ix_data.encryptedUtxos,
              ix_data.merkleTreeIndex
              ).accounts(
                  {
                    signingAddress: relayer.publicKey,
                    verifierState: pdas.verifierStatePubkey,
                    systemProgram: SystemProgram.programId,
                    merkleTree: merkle_tree_pubkey,
                    programMerkleTree:  merkleTreeProgram.programId,
                  }
              ).signers([relayer]).transaction()
            await provider.sendAndConfirm(tx, [relayer])
      } catch(e) {
        console.log(e)
        process.exit()
      }

      checkVerifierStateAccountCreated({
        connection:connection,
        pda: pdas.verifierStatePubkey,
        ix_data,
        relayer_pubkey:relayer.publicKey
      })

      await executeXComputeTransactions({
        number_of_transactions: nr_tx,
        signer: relayer,
        pdas: pdas,
        program: verifierProgram,
        provider:provider
      })
      var verifierStatePrior = await connection.getAccountInfo(
        pdas.verifierStatePubkey
      )
      try  {
        const tx = await verifierProgram.methods.createVerifierState(
              ix_data.proofAbc,
              ix_data.rootHash,
              ix_data.amount,
              ix_data.txIntegrityHash,
              ix_data.nullifier0,
              ix_data.nullifier1,
              ix_data.leafRight,
              ix_data.leafLeft,
              ix_data.recipient,
              ix_data.extAmount,
              ix_data.relayer,
              ix_data.fee,
              ix_data.encryptedUtxos,
              ix_data.merkleTreeIndex
              ).accounts(
                  {
                    signingAddress: relayer.publicKey,
                    verifierState: pdas.verifierStatePubkey,
                    systemProgram: SystemProgram.programId,
                    merkleTree: merkle_tree_pubkey,
                    programMerkleTree:  merkleTreeProgram.programId,
                  }
              ).signers([relayer]).transaction()
            await provider.sendAndConfirm(tx, [relayer])
      } catch(e) {
        assert(e.logs[2] == 'Program log: AnchorError thrown in programs/verifier_program/src/groth16_verifier/create_verifier_state.rs:62. Error Code: VerifierStateAlreadyInitialized. Error Number: 6008. Error Message: VerifierStateAlreadyInitialized.')
      }
    var verifierState = await connection.getAccountInfo(
      pdas.verifierStatePubkey
    )
    const accountPriorUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierStatePrior.data);

    let accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierState.data);
    console.log(`${accountPriorUpdate.currentInstructionIndex} != ${accountAfterUpdate.currentInstructionIndex}`)
    assert(accountPriorUpdate.currentInstructionIndex.toString() == accountAfterUpdate.currentInstructionIndex.toString());

  })

  it("Signer is consistent", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()
    let merkle_tree_pubkey = MERKLE_TREE_KEY
    // let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    let rent_verifier = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    provider.wallet.payer = relayer
    let nr_tx = 10;
    let tx_cost = (nr_tx + 1) * 5000
    // const burnerUserAccount = await newAccountWithLamports(connection)
    let merkleTree = await light.buildMerkelTree(provider.connection);


    let pdas = getPdaAddresses({
      tx_integrity_hash: IX_DATA.txIntegrityHash,
      nullifier0: IX_DATA.nullifier0,
      nullifier1: IX_DATA.nullifier1,
      leafLeft: IX_DATA.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(relayer.publicKey)
    var userInfoStart = await connection.getAccountInfo(origin.publicKey)

      var verifierStatePrior = await connection.getAccountInfo(
        pdas.verifierStatePubkey
      )
      try {
        await executeXComputeTransactions({
          number_of_transactions: nr_tx,
          signer: origin,
          pdas: pdas,
          program: verifierProgram,
          provider:provider
        })
      } catch(e) {
        assert(e.logs[2] == 'Program log: AnchorError caused by account: signing_address. Error Code: ConstraintAddress. Error Number: 2012. Error Message: An address constraint was violated.')
      }

    var verifierState = await connection.getAccountInfo(
      pdas.verifierStatePubkey
    )

    const accountPriorUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierStatePrior.data);
    let accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierState.data);
    console.log(`${accountPriorUpdate.currentInstructionIndex} == ${accountAfterUpdate.currentInstructionIndex}`)
    assert(accountPriorUpdate.currentInstructionIndex.toString() == accountAfterUpdate.currentInstructionIndex.toString());

  })

  it("Invoke last transaction with wrong instruction index", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()
    let merkle_tree_pubkey = MERKLE_TREE_KEY
    let authority = AUTHORITY
    let preInsertedLeavesIndex = PRE_INSERTED_LEAVES_INDEX
    let hure = SIGNER
    // let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    let rent_verifier = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    provider.wallet.payer = relayer
    let nr_tx = 10;
    let tx_cost = (nr_tx + 1) * 5000
    // const burnerUserAccount = await newAccountWithLamports(connection)
    let merkleTree = await light.buildMerkelTree(provider.connection);


    let pdas = getPdaAddresses({
      tx_integrity_hash: IX_DATA.txIntegrityHash,
      nullifier0: IX_DATA.nullifier0,
      nullifier1: IX_DATA.nullifier1,
      leafLeft: IX_DATA.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(relayer.publicKey)
    var userInfoStart = await connection.getAccountInfo(origin.publicKey)

    var verifierStatePrior = await connection.getAccountInfo(
      pdas.verifierStatePubkey
    )

    try {
      const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
            ).accounts(
                {
                  signingAddress: relayer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // merkleTreeUpdateState:pdas.merkleTreeUpdateState,
                  systemProgram: SystemProgram.programId,
                  programMerkleTree: merkleTreeProgram.programId,
                  rent: DEFAULT_PROGRAMS.rent,
                  nullifier0Pda: pdas.nullifier0PdaPubkey,
                  nullifier1Pda: pdas.nullifier1PdaPubkey,
                  twoLeavesPda: pdas.leavesPdaPubkey,
                  escrowPda: pdas.escrowPdaPubkey,
                  merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
                  userAccount: origin.publicKey,
                  merkleTree: merkle_tree_pubkey,
                  feeEscrowState: pdas.feeEscrowStatePubkey,
                  merkleTreeProgram:  merkleTreeProgram.programId,
                  preInsertedLeavesIndex: preInsertedLeavesIndex,
                  authority: authority
                }
              ).preInstructions([
                SystemProgram.transfer({
                  fromPubkey: relayer.publicKey,
                  toPubkey: authority,
                  lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
                })
              ]).signers([relayer]).rpc()

      } catch(e) {
        assert(e.error.origin == 'signing_address');
        assert(e.error.errorCode.code == 'ConstraintAddress');
    }
    try {
      const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
            ).accounts(
                {
                  signingAddress: SIGNER.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // merkleTreeUpdateState:pdas.merkleTreeUpdateState,
                  systemProgram: SystemProgram.programId,
                  programMerkleTree: merkleTreeProgram.programId,
                  rent: DEFAULT_PROGRAMS.rent,
                  nullifier0Pda: pdas.nullifier0PdaPubkey,
                  nullifier1Pda: pdas.nullifier1PdaPubkey,
                  twoLeavesPda: pdas.leavesPdaPubkey,
                  escrowPda: pdas.escrowPdaPubkey,
                  merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
                  userAccount: origin.publicKey,
                  merkleTree: merkle_tree_pubkey,
                  feeEscrowState: pdas.feeEscrowStatePubkey,
                  merkleTreeProgram:  merkleTreeProgram.programId,
                  preInsertedLeavesIndex: preInsertedLeavesIndex,
                  authority: authority
                }
              ).preInstructions([
                SystemProgram.transfer({
                  fromPubkey: SIGNER.publicKey,
                  toPubkey: authority,
                  lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
                })
              ]).signers([SIGNER]).rpc()

      } catch(e) {
        assert(e.error.errorCode.code == 'NotLastTransactionState');
    }
    var verifierState = await connection.getAccountInfo(
      pdas.verifierStatePubkey
    )

    const accountPriorUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierStatePrior.data);
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierState.data);
    console.log(`${accountPriorUpdate.currentInstructionIndex} == ${accountAfterUpdate.currentInstructionIndex}`)
    assert(accountPriorUpdate.currentInstructionIndex.toString() == accountAfterUpdate.currentInstructionIndex.toString());

  })


   it("Last tx deposit with wrong accounts", async () => {
      const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
      const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
      var signer
      var pdas
      var leavesPdas = []
      var utxos = []

      //
      // *
      // * test deposit
      // *
      //

      let merkleTree = await light.buildMerkelTree(provider.connection);

      let Keypair = new light.Keypair()

      for (var i= 0; i < 1; i++) {
        try {
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
            relayerFee,
            lastTx: false
          })
          leavesPdas.push({ isSigner: false, isWritable: true, pubkey: res[0]})
          utxos.push(res[1])
          signer = res[2]
          pdas = res[3]
        } catch(e) {
          console.log(e)
        }


      }
      let escrowAccountInfo = await provider.connection.getAccountInfo(pdas.feeEscrowStatePubkey)
      console.log("escrowAccountInfo: ", escrowAccountInfo)
      // wrong recipient
      const maliciousRecipient = await newProgramOwnedAccount({ connection: provider.connection,owner: merkleTreeProgram}) // new anchor.web3.Account()
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: maliciousRecipient.publicKey,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'merkle_tree_pda_token')
      }
      // try with unregistered merkle tree
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
            merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'merkle_tree_pda_token')
      }
      // try with wrong PRE_INSERTED_LEAVES_INDEX
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'pre_inserted_leaves_index')
      }

      // try with wrong leaves account
      const maliciousLeaf = solana.PublicKey.findProgramAddressSync([Buffer.from(new Uint8Array(32).fill(4)), anchor.utils.bytes.utf8.encode("leaves")],
      merkleTreeProgram.programId)[0]
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: maliciousLeaf,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'two_leaves_pda')
      }

      // try with wrong leaves account
      const maliciousNullifier = solana.PublicKey.findProgramAddressSync([Buffer.from(new Uint8Array(32).fill(4)), anchor.utils.bytes.utf8.encode("nf")],
      merkleTreeProgram.programId)[0]

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: maliciousNullifier,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'nullifier0_pda')
      }

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: maliciousNullifier,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'nullifier1_pda')
      }
      // different escrow account
      const maliciousEscrow = solana.PublicKey.findProgramAddressSync([Buffer.from(new Uint8Array(32).fill(5)), anchor.utils.bytes.utf8.encode("fee_escrow")],
      merkleTreeProgram.programId)[0]
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: maliciousEscrow,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'fee_escrow_state')
      }

      const maliciousSigner = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
        ).accounts(
          {
            signingAddress: maliciousSigner.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: maliciousSigner.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([maliciousSigner]).rpc()
      } catch(e) {
        assert(e.error.origin == 'signing_address')
      }
    })


  it("wrong tx txIntegrityHash", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()
    let merkle_tree_pubkey = MERKLE_TREE_KEY
    // let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
    let rent_verifier = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    provider.wallet.payer = relayer
    let nr_tx = 10;
    let tx_cost = (nr_tx + 1) * 5000
    // const burnerUserAccount = await newAccountWithLamports(connection)
    let merkleTree = await light.buildMerkelTree(provider.connection);

    let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)
    let deposit_utxo2 = new light.Utxo(BigNumber.from(amount), Keypair)

    let inputUtxos = [new light.Utxo(), new light.Utxo()]
    let outputUtxos = [deposit_utxo1, deposit_utxo2 ]

    const data = await light.getProof(
      inputUtxos,
      outputUtxos,
      merkleTree,
      deposit_utxo1.amount.add(deposit_utxo2.amount),
      U64(0),
      MERKLE_TREE_PDA_TOKEN.toBase58(),
      relayer.publicKey.toBase58(),
      'DEPOSIT',
      encryptionKeypair
    )
    let ix_data = parse_instruction_data_bytes(data);

    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })

    // wrong ext amount
    try  {
      const tx = await verifierProgram.methods.createVerifierState(
            ix_data.proofAbc,
            ix_data.rootHash,
            ix_data.amount,
            ix_data.txIntegrityHash, // replaced tx_integrity_hash
            ix_data.nullifier0,
            ix_data.nullifier1,
            ix_data.leafRight,
            ix_data.leafLeft,
            ix_data.recipient,
            new Uint8Array(8).fill(1),
            ix_data.relayer,
            ix_data.fee,
            ix_data.encryptedUtxos,
            ix_data.merkleTreeIndex
            ).accounts(
                {
                  signingAddress: relayer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  systemProgram: SystemProgram.programId,
                  merkleTree: merkle_tree_pubkey,
                  programMerkleTree:  merkleTreeProgram.programId,
                }
            ).signers([relayer]).rpc()
    } catch(e) {
      assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    }
    // wrong relayer
    try  {
      const tx = await verifierProgram.methods.createVerifierState(
            ix_data.proofAbc,
            ix_data.rootHash,
            ix_data.amount,
            ix_data.txIntegrityHash, // replaced tx_integrity_hash
            ix_data.nullifier0,
            ix_data.nullifier1,
            ix_data.leafRight,
            ix_data.leafLeft,
            ix_data.recipient,
            ix_data.extAmount,
            ix_data.relayer,
            ix_data.fee,
            ix_data.encryptedUtxos,
            ix_data.merkleTreeIndex
            ).accounts(
                {
                  signingAddress: SIGNER.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  systemProgram: SystemProgram.programId,
                  merkleTree: merkle_tree_pubkey,
                  programMerkleTree:  merkleTreeProgram.programId,
                }
            ).signers([SIGNER]).rpc()
    } catch(e) {
      assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    }
    // wrong fee
    try  {
      const tx = await verifierProgram.methods.createVerifierState(
            ix_data.proofAbc,
            ix_data.rootHash,
            ix_data.amount,
            ix_data.txIntegrityHash, // replaced tx_integrity_hash
            ix_data.nullifier0,
            ix_data.nullifier1,
            ix_data.leafRight,
            ix_data.leafLeft,
            ix_data.recipient,
            ix_data.extAmount,
            ix_data.relayer,
            new Uint8Array(8).fill(1),
            ix_data.encryptedUtxos,
            ix_data.merkleTreeIndex
            ).accounts(
                {
                  signingAddress: relayer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  systemProgram: SystemProgram.programId,
                  merkleTree: merkle_tree_pubkey,
                  programMerkleTree:  merkleTreeProgram.programId,
                }
            ).signers([relayer]).rpc()
    } catch(e) {
      assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    }
    // wrong utxos
    try  {
      const tx = await verifierProgram.methods.createVerifierState(
            ix_data.proofAbc,
            ix_data.rootHash,
            ix_data.amount,
            ix_data.txIntegrityHash, // replaced tx_integrity_hash
            ix_data.nullifier0,
            ix_data.nullifier1,
            ix_data.leafRight,
            ix_data.leafLeft,
            ix_data.recipient,
            ix_data.extAmount,
            ix_data.relayer,
            ix_data.fee,
            new Uint8Array(222).fill(1),
            ix_data.merkleTreeIndex
            ).accounts(
                {
                  signingAddress: relayer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  systemProgram: SystemProgram.programId,
                  merkleTree: merkle_tree_pubkey,
                  programMerkleTree:  merkleTreeProgram.programId,
                }
            ).signers([relayer]).rpc()
    } catch(e) {
      assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    }

    // wrong merkle tree index fails for index out of bounds rn
    // try  {
    //   const tx = await verifierProgram.methods.createVerifierState(
    //         ix_data.proofAbc,
    //         ix_data.rootHash,
    //         ix_data.amount,
    //         ix_data.txIntegrityHash, // replaced tx_integrity_hash
    //         ix_data.nullifier0,
    //         ix_data.nullifier1,
    //         ix_data.leafRight,
    //         ix_data.leafLeft,
    //         ix_data.recipient,
    //         ix_data.extAmount,
    //         ix_data.relayer,
    //         ix_data.fee,
    //         ix_data.encryptedUtxos,
    //         new Uint8Array(1).fill(1)
    //         ).accounts(
    //             {
    //               signingAddress: relayer.publicKey,
    //               verifierState: pdas.verifierStatePubkey,
    //               systemProgram: SystemProgram.programId,
    //               merkleTree: merkle_tree_pubkey,
    //               programMerkleTree:  merkleTreeProgram.programId,
    //             }
    //         ).signers([relayer]).rpc()
    // } catch(e) {
    //   console.log(e)
    //   assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    // }

    // wrong recipient
    try  {
      const tx = await verifierProgram.methods.createVerifierState(
            ix_data.proofAbc,
            ix_data.rootHash,
            ix_data.amount,
            ix_data.txIntegrityHash, // replaced tx_integrity_hash
            ix_data.nullifier0,
            ix_data.nullifier1,
            ix_data.leafRight,
            ix_data.leafLeft,
            SIGNER.publicKey.toBytes(),
            ix_data.extAmount,
            ix_data.relayer,
            ix_data.fee,
            ix_data.encryptedUtxos,
            ix_data.merkleTreeIndex
            ).accounts(
                {
                  signingAddress: relayer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  systemProgram: SystemProgram.programId,
                  merkleTree: merkle_tree_pubkey,
                  programMerkleTree:  merkleTreeProgram.programId,
                }
            ).signers([relayer]).rpc()
    } catch(e) {
      assert(e.error.errorCode.code == 'WrongTxIntegrityHash')
    }
  })
  let UTXOS
  let MERKLE_TREE_OLD
  it("Double Spend", async () => {
      const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
      const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      var leavesPdas = []
      var utxos = []

      //
      // *
      // * test deposit
      // *
      //

      let merkleTree = await light.buildMerkelTree(provider.connection);
      MERKLE_TREE_OLD = merkleTree
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
      UTXOS = utxos
      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:userAccount,
        merkleTreeProgram: merkleTreeProgram,
        leavesPdas,
        merkleTree,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider
      });


      // *
      // * test withdrawal
      // *
      // *
      // *

      // new lightTransaction
      // generate utxos
      //
      var leavesPdasWithdrawal = []
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection);
      let deposit_utxo1 = utxos[0][0];
      let deposit_utxo2 = utxos[0][1];
      deposit_utxo1.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo1.getCommitment()._hex)
      deposit_utxo2.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo2.getCommitment()._hex)

      let relayer = await newAccountWithLamports(provider.connection);
      let relayer_recipient = new anchor.web3.Account();
      provider.payer = relayer
      let inputUtxosWithdrawal = []
      // TODO // DEBUG: getting invalid proof when selecting utxo with index 0
      if (deposit_utxo1.index == 1) {
        inputUtxosWithdrawal = [deposit_utxo1, new light.Utxo()] // 38241198
      } else {
        inputUtxosWithdrawal = [deposit_utxo2, new light.Utxo()] // 38241198
      }
      let outputUtxosWithdrawal = [new light.Utxo(), new light.Utxo() ]

      const externalAmountBigNumber: BigNumber = BigNumber.from(relayerFee.toString())
      .add(
        outputUtxosWithdrawal.reduce(
          (sum, utxo) => sum.add(utxo.amount),
          BigNumber.from(0),
        ),
      )
      .sub(
        inputUtxosWithdrawal.reduce((sum, utxo) => sum.add(utxo.amount), BigNumber.from(0)),
      )
      console.log("externalAmountBigNumber ", externalAmountBigNumber.toString())
      var dataWithdrawal = await light.getProof(
        inputUtxosWithdrawal,
        outputUtxosWithdrawal,
        merkleTreeWithdrawal,
        externalAmountBigNumber,
        relayerFee,
        recipientWithdrawal.publicKey.toBase58(),
        relayer.publicKey.toBase58(),
        'WITHDRAWAL',
        encryptionKeypair
      )



      let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);

      let pdasWithdrawal = getPdaAddresses({
        tx_integrity_hash: ix_dataWithdrawal.txIntegrityHash,
        nullifier0: ix_dataWithdrawal.nullifier0,
        nullifier1: ix_dataWithdrawal.nullifier1,
        leafLeft: ix_dataWithdrawal.leafLeft,
        merkleTreeProgram,
        verifierProgram
      })


      let resWithdrawalTransact = await transact({
        connection: provider.connection,
        ix_data: ix_dataWithdrawal,
        pdas: pdasWithdrawal,
        origin: MERKLE_TREE_PDA_TOKEN,
        signer: relayer,
        recipient: recipientWithdrawal.publicKey,
        relayer_recipient,
        mode: "withdrawal",
        verifierProgram,
        merkleTreeProgram,
        authority: AUTHORITY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider,
        relayerFee
      })

      let failed = false
      try {

        let tx23 = await transact({
          connection: provider.connection,
          ix_data: ix_dataWithdrawal,
          pdas: pdasWithdrawal,
          origin: MERKLE_TREE_PDA_TOKEN,
          signer: relayer,
          recipient: recipientWithdrawal.publicKey,
          relayer_recipient,
          mode: "withdrawal",
          verifierProgram,
          merkleTreeProgram,
          authority: AUTHORITY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
          merkle_tree_pubkey: MERKLE_TREE_KEY,
          provider,
          relayerFee
        })
      } catch (e) {
        // console.log(e)
        // console.log(e.programErrorStack)
        // console.log(e.programErrorStack[0].toBase58())
        // console.log(e.programErrorStack[1].toBase58())
        // console.log(e.programErrorStack[2].toBase58())
        // console.log(pdasWithdrawal.nullifier0PdaPubkey.toBase58())
        // console.log(pdasWithdrawal.nullifier1PdaPubkey.toBase58())
        failed = true
      }
      assert(failed, "double spend did not fail");
    })

  it("Last Tx Withdrawal false inputs", async () => {
      const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
      const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      var leavesPdas = []
      var utxos = []


      // *
      // * test withdrawal
      // *
      // *
      // *

      console.log("Last Tx Withdrawal false inputs")
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection);

      let signer = await newAccountWithLamports(provider.connection);
      let relayer_recipient = new anchor.web3.Account();
      provider.payer = signer
      let inputUtxosWithdrawal = [UTXOS[1][1], new light.Utxo()] // 38241198

      let outputUtxosWithdrawal = [new light.Utxo(), new light.Utxo() ]

      const externalAmountBigNumber: BigNumber = BigNumber.from(relayerFee.toString())
      .add(
        outputUtxosWithdrawal.reduce(
          (sum, utxo) => sum.add(utxo.amount),
          BigNumber.from(0),
        ),
      )
      .sub(
        inputUtxosWithdrawal.reduce((sum, utxo) => sum.add(utxo.amount), BigNumber.from(0)),
      )
      console.log("externalAmountBigNumber ", externalAmountBigNumber.toString())
      var dataWithdrawal = await light.getProof(
        inputUtxosWithdrawal,
        outputUtxosWithdrawal,
        merkleTreeWithdrawal,
        externalAmountBigNumber,
        relayerFee,
        recipientWithdrawal.publicKey.toBase58(),
        signer.publicKey.toBase58(),
        'WITHDRAWAL',
        encryptionKeypair
      )



      let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);

      let pdasWithdrawal = getPdaAddresses({
        tx_integrity_hash: ix_dataWithdrawal.txIntegrityHash,
        nullifier0: ix_dataWithdrawal.nullifier0,
        nullifier1: ix_dataWithdrawal.nullifier1,
        leafLeft: ix_dataWithdrawal.leafLeft,
        merkleTreeProgram,
        verifierProgram
      })
      let pdas = pdasWithdrawal

      let failed = false
      try {

        let tx23 = await transact({
          connection: provider.connection,
          ix_data: ix_dataWithdrawal,
          pdas: pdasWithdrawal,
          origin: MERKLE_TREE_PDA_TOKEN,
          signer: signer,
          recipient: recipientWithdrawal.publicKey,
          relayer_recipient,
          mode: "withdrawal",
          verifierProgram,
          merkleTreeProgram,
          authority: AUTHORITY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
          merkle_tree_pubkey: MERKLE_TREE_KEY,
          provider,
          relayerFee,
          lastTx: false
        })
      } catch (e) {
        console.log(e)
        // console.log(e.programErrorStack)
        // console.log(e.programErrorStack[0].toBase58())
        // console.log(e.programErrorStack[1].toBase58())
        // console.log(e.programErrorStack[2].toBase58())
        // console.log(pdasWithdrawal.nullifier0PdaPubkey.toBase58())
        // console.log(pdasWithdrawal.nullifier1PdaPubkey.toBase58())
        failed = true
      }
      console.log("here")
      // signingAddress: signer.publicKey,
      // nullifier0Pda: pdas.nullifier0PdaPubkey,
      // nullifier1Pda: pdas.nullifier1PdaPubkey,
      // twoLeavesPda: pdas.leavesPdaPubkey,
      // verifierState: pdas.verifierStatePubkey,
      // programMerkleTree: merkleTreeProgram.programId,
      // systemProgram: SystemProgram.programId,
      // rent: DEFAULT_PROGRAMS.rent,
      // recipient: recipientWithdrawal.publicKey,
      // relayer_recipient: relayer_recipient.publicKey,
      // merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
      // merkleTree: MERKLE_TREE_KEY,
      // preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      // authority: AUTHORITY
      // let escrowAccountInfo = await provider.connection.getAccountInfo(pdas.feeEscrowStatePubkey)
      // console.log("escrowAccountInfo: ", escrowAccountInfo)
      // wrong recipient
      const maliciousRecipient = await newProgramOwnedAccount({ connection: provider.connection,owner: merkleTreeProgram}) // new anchor.web3.Account()
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: maliciousRecipient.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'recipient')
      }
      // try with unregistered merkle tree
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
            merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'merkle_tree_pda_token')
      }
      // try with wrong PRE_INSERTED_LEAVES_INDEX
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'pre_inserted_leaves_index')
      }

      // try with wrong leaves account
      const maliciousLeaf = solana.PublicKey.findProgramAddressSync([Buffer.from(new Uint8Array(32).fill(4)), anchor.utils.bytes.utf8.encode("leaves")],
      merkleTreeProgram.programId)[0]
      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: maliciousLeaf,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'two_leaves_pda')
      }

      // try with wrong leaves account
      const maliciousNullifier = solana.PublicKey.findProgramAddressSync([Buffer.from(new Uint8Array(32).fill(4)), anchor.utils.bytes.utf8.encode("nf")],
      merkleTreeProgram.programId)[0]

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: maliciousNullifier,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'nullifier0_pda')
      }

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: maliciousNullifier,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
      } catch(e) {
        assert(e.error.origin == 'nullifier1_pda')
      }

      const maliciousSigner = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      try {
        const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
          {
            signingAddress: maliciousSigner.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            recipient: recipientWithdrawal.publicKey,
            relayerRecipient: relayer_recipient.publicKey,
            merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
            merkleTree: MERKLE_TREE_KEY,
            preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
            authority: AUTHORITY
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: maliciousSigner.publicKey,
            toPubkey: AUTHORITY,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([maliciousSigner]).rpc()
      } catch(e) {
        assert(e.error.origin == 'signing_address')
      }

    })


  it("Wrong root & merkle proof", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let Keypair = new light.Keypair()

    var leavesPdas = []
    var utxos = []
    let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)

    // inserting malicious commitment into local Merkle tree
    MERKLE_TREE_OLD.update(2, deposit_utxo1.getCommitment()._hex)


    // *
    // * test withdrawal
    // *
    // *
    // *

    // new lightTransaction
    // generate utxos
    //
    var leavesPdasWithdrawal = []
    const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection);

    let relayer = await newAccountWithLamports(provider.connection);
    let relayer_recipient = new anchor.web3.Account();
    provider.payer = relayer
    let inputUtxosWithdrawal = []
    inputUtxosWithdrawal = [deposit_utxo1, new light.Utxo()] // 38241198

    let outputUtxosWithdrawal = [new light.Utxo(), new light.Utxo() ]

    const externalAmountBigNumber: BigNumber = BigNumber.from(relayerFee.toString())
    .add(
      outputUtxosWithdrawal.reduce(
        (sum, utxo) => sum.add(utxo.amount),
        BigNumber.from(0),
      ),
    )
    .sub(
      inputUtxosWithdrawal.reduce((sum, utxo) => sum.add(utxo.amount), BigNumber.from(0)),
    )
    console.log("externalAmountBigNumber ", externalAmountBigNumber.toString())
    var dataWithdrawal = await light.getProof(
      inputUtxosWithdrawal,
      outputUtxosWithdrawal,
      MERKLE_TREE_OLD,
      externalAmountBigNumber,
      relayerFee,
      recipientWithdrawal.publicKey.toBase58(),
      relayer.publicKey.toBase58(),
      'WITHDRAWAL',
      encryptionKeypair
    )

    let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);

    let pdasWithdrawal = getPdaAddresses({
      tx_integrity_hash: ix_dataWithdrawal.txIntegrityHash,
      nullifier0: ix_dataWithdrawal.nullifier0,
      nullifier1: ix_dataWithdrawal.nullifier1,
      leafLeft: ix_dataWithdrawal.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })

    try {
      let resWithdrawalTransact = await transact({
        connection: provider.connection,
        ix_data: ix_dataWithdrawal,
        pdas: pdasWithdrawal,
        origin: MERKLE_TREE_PDA_TOKEN,
        signer: relayer,
        recipient: recipientWithdrawal.publicKey,
        relayer_recipient,
        mode: "withdrawal",
        verifierProgram,
        merkleTreeProgram,
        authority: AUTHORITY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider,
        relayerFee
      })
    } catch (e) {
      assert(e.logs.indexOf('Program log: Did not find root.') != -1)
    }
  })


  it("Dynamic Shielded transaction", async () => {
      const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
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

      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:userAccount,
        merkleTreeProgram: merkleTreeProgram,
        leavesPdas,
        merkleTree,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider
      });


      // *
      // * test withdrawal
      // *
      // *
      // *

      // new lightTransaction
      // generate utxos
      //
      var leavesPdasWithdrawal = []
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection);
      let deposit_utxo1 = utxos[0][0];
      let deposit_utxo2 = utxos[0][1];
      deposit_utxo1.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo1.getCommitment()._hex)
      deposit_utxo2.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo2.getCommitment()._hex)

      let relayer = await newAccountWithLamports(provider.connection);
      let relayer_recipient = new anchor.web3.Account();

      let inputUtxosWithdrawal = []
      if (deposit_utxo1.index == 1) {
        inputUtxosWithdrawal = [deposit_utxo1, new light.Utxo()] // 38241198
      } else {
        inputUtxosWithdrawal = [deposit_utxo2, new light.Utxo()] // 38241198
      }
      let outputUtxosWithdrawal = [new light.Utxo(), new light.Utxo() ]

      const externalAmountBigNumber: BigNumber = BigNumber.from(relayerFee.toString())
      .add(
        outputUtxosWithdrawal.reduce(
          (sum, utxo) => sum.add(utxo.amount),
          BigNumber.from(0),
        ),
      )
      .sub(
        inputUtxosWithdrawal.reduce((sum, utxo) => sum.add(utxo.amount), BigNumber.from(0)),
      )
      console.log("externalAmountBigNumber ", externalAmountBigNumber.toString())
      var dataWithdrawal = await light.getProof(
        inputUtxosWithdrawal,
        outputUtxosWithdrawal,
        merkleTreeWithdrawal,
        externalAmountBigNumber,
        relayerFee,
        recipientWithdrawal.publicKey.toBase58(),
        relayer.publicKey.toBase58(),
        'WITHDRAWAL',
        encryptionKeypair
      )

      let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);
      let pdasWithdrawal = getPdaAddresses({
        tx_integrity_hash: ix_dataWithdrawal.txIntegrityHash,
        nullifier0: ix_dataWithdrawal.nullifier0,
        nullifier1: ix_dataWithdrawal.nullifier1,
        leafLeft: ix_dataWithdrawal.leafLeft,
        merkleTreeProgram,
        verifierProgram
      })
      let resWithdrawalTransact = await transact({
        connection: provider.connection,
        ix_data: ix_dataWithdrawal,
        pdas: pdasWithdrawal,
        origin: MERKLE_TREE_PDA_TOKEN,
        signer: relayer,
        recipient: recipientWithdrawal.publicKey,
        relayer_recipient,
        mode: "withdrawal",
        verifierProgram,
        merkleTreeProgram,
        authority: AUTHORITY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider,
        relayerFee
      })
      leavesPdasWithdrawal.push({
        isSigner: false,
        isWritable: true,
        pubkey: resWithdrawalTransact
      })
      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:relayer,
        merkleTreeProgram,
        leavesPdas: leavesPdasWithdrawal,
        merkleTree: merkleTreeWithdrawal,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider
      });

    })

});
