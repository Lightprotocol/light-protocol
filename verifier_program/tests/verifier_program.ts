import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber } from 'ethers'
const { poseidonHash } = require('./utils/poseidonHash')
const {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  recipient,
  relayer,
  testUuid,
  testTimeout,
  inputUtxoAmount,
  outputUtxoAmount,
  relayerFee,
  testInputUtxo,
  testOutputUtxo
} = require ('./utils/testUtxos');
import _ from "lodash"

const PREPARED_INPUTS_TX_COUNT = 36
const MILLER_LOOP_TX_COUNT = 40
const FINAL_EXPONENTIATION_TX_COUNT = 19
const MERKLE_TREE_UPDATE_TX_COUNT = 78

//
// const Utxo = require("./utils/utxo");
// const prepareTransaction = require("./utils/prepareTransaction");
const MerkleTree = require("./utils/merkleTree");

const light = require('@darjusch/light-protocol-sdk');

const newAddressWithLamports = async (connection,address = new anchor.web3.Account().publicKey, lamports = 1e10) => {

  let retries = 30
  await connection.requestAirdrop(address, lamports)
  for (;;) {
    await sleep(500)
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(address))) {
      console.log(`Airdropped ${lamports} to ${address.toBase58()}`)
      return address
    }
    if (--retries <= 0) {
      break
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`)
}
const newAccountWithLamports = async (connection,account = new anchor.web3.Account(),verifierProgram, lamports = 1e9) => {
  let retries = 30
  if (verifierProgram != undefined) {
    let payer = new solana.Account()

    await connection.requestAirdrop(payer.publicKey, 1e10)
    await sleep(500)
    const tx = new solana.Transaction().add(
      solana.SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: account.publicKey,
        space: 0,
        lamports: await connection.getMinimumBalanceForRentExemption(0),
        programId: verifierProgram.programId,
      })
    ).add(
      solana.SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: account.publicKey,
        lamports: 1e9,
      })
    )
    tx.feePayer = payer.publicKey
    tx.recentBlockhash = await connection.getRecentBlockhash();
    // tx.sign([payer])
    let x = await solana.sendAndConfirmTransaction(
          connection,
          tx,
          [payer, account],
          {
            commitment: 'singleGossip',
            preflightCommitment: 'singleGossip',
          },
      );
      return account;
  }


  await connection.requestAirdrop(account.publicKey, lamports)

  for (;;) {
    await sleep(500)
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(account.publicKey))) {
      return account
    }
    if (--retries <= 0) {
      break
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`)
}
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function assert_eq(
  value0: unknown,
  value1: unknown,
  message: string
) {

  if (value0.length !== value1.length) {
    console.log("value0: ", value0)
    console.log("value1: ", value1)
    throw Error("Length of asserted values does not match");
  }
  for (var i = 0; i < value0.length; i++) {
    if (value0[i] !== value1[i]) {
      throw Error(message);
    }
  }

}
import { MerkleTreeProgram, IDL } from "../target/types/merkle_tree_program";
const constants:any = {};

const TYPE_PUBKEY = { array: [ 'u8', 32 ] };
const TYPE_SEED = {defined: "&[u8]"};
const TYPE_INIT_DATA = { array: [ 'u8', 642 ] };
IDL.constants.map((item) => {
  if(_.isEqual(item.type, TYPE_SEED)) {
    constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
  } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
  {
    constants[item.name] = JSON.parse(item.value)
  }
});



const read_and_parse_instruction_data_bytes = ()  => {
  let file = fs.readFileSync('tests/deposit.txt','utf8');
  // let file = await fs.readFile("deposit.txt", function(err, fd) {
  //  if (err) {
  //     return console.error(err);
  //  }
   console.log("File opened successfully!");
   var data = JSON.parse(file.toString());
   var partsOfStr = data.bytes[0].split(',');
   let bytes = []
   partsOfStr.map((byte, index)=> {
     if (index > 8) {
       bytes[index] = Number(byte);

     }
   })
   bytes = bytes.slice(9,)

   let ix_data = {
     rootHash:          bytes.slice(0,32),
     amount:             bytes.slice(32,64),
     txIntegrityHash:  bytes.slice(64,96),
     nullifier0:         bytes.slice(96,128),
     nullifier1:         bytes.slice(128,160),
     leafRight:         bytes.slice(160,192),
     leafLeft:          bytes.slice(192,224),
     proofAbc:        bytes.slice(224,480),
     // relayer_fee:        bytes.slice(264,272),
     // ext_sol_amount:     bytes.slice(272,304),
     // verifier_index:     bytes.slice(304,312),
     // merkleTreeIndex:  bytes.slice(312,320),
     recipient:          bytes.slice(480,512),
     extAmount:         bytes.slice(512,520),
     relayer:            bytes.slice(520, 552),
     fee:                bytes.slice(552, 560),
     merkleTreePdaPubkey:bytes.slice(560, 592),
     merkleTreeIndex:  bytes.slice(592,593),
     encryptedUtxos:    bytes.slice(593,593+222),
   }
   return {ix_data, bytes};
}

function parse_instruction_data_bytes(data) {
  console.log(data)
   let ix_data = {
     rootHash:          data.data.publicInputsBytes.slice(0,32),
     amount:             data.data.publicInputsBytes.slice(32,64),
     txIntegrityHash:  data.data.publicInputsBytes.slice(64,96),
     nullifier0:         data.data.publicInputsBytes.slice(96,128),
     nullifier1:         data.data.publicInputsBytes.slice(128,160),
     leafRight:         data.data.publicInputsBytes.slice(160,192),
     leafLeft:          data.data.publicInputsBytes.slice(192,224),
     proofAbc:        data.data.proofBytes,
     // relayer_fee:        bytes.slice(264,272),
     // ext_sol_amount:     bytes.slice(272,304),
     // verifier_index:     bytes.slice(304,312),
     // merkleTreeIndex:  bytes.slice(312,320),
     recipient:          data.data.extDataBytes.slice(0,32),
     extAmount:         data.data.extAmount,
     relayer:            data.data.extDataBytes.slice(40,72),
     fee:                data.data.extDataBytes.slice(72,80),
     merkleTreePdaPubkey:data.data.extDataBytes.slice(80,112),
     merkleTreeIndex:     data.data.extDataBytes.slice(112,113),
     encryptedUtxos:    data.data.extDataBytes.slice(113,335),
   }
   return ix_data;
}
async function readAndParseAccountDataMerkleTreeTmpState({
  connection,
  pda
}) {
  var userAccountInfo = await connection.getAccountInfo(
        pda
      )

    let object = {
        is_initialized: userAccountInfo.data[0],
        account_type: userAccountInfo.data[1],
        current_instruction_index: U64.readLE(userAccountInfo.data.slice(2,10),0).toString(),
        found_root: userAccountInfo.data[10],                     //0
        merkle_tree_pda_pubkey: Array.prototype.slice.call(userAccountInfo.data.slice(11,43)),       //2
        relayer: Array.prototype.slice.call(userAccountInfo.data.slice(43,75)),     //3
        root_hash: Array.prototype.slice.call(userAccountInfo.data.slice(75,107)),

        state: Array.prototype.slice.call(userAccountInfo.data.slice(107,203)),
        current_round: U64.readLE(userAccountInfo.data.slice(235,243),0).toString(),
        current_round_index: U64.readLE(userAccountInfo.data.slice(243,251),0).toString(),
        current_index: U64.readLE(userAccountInfo.data.slice(251,259),0).toString(),
        current_level: U64.readLE(userAccountInfo.data.slice(259,267),0).toString(),
        current_level_hash: Array.prototype.slice.call(userAccountInfo.data.slice(235,267)),

        node_left: Array.prototype.slice.call(userAccountInfo.data.slice(267,299)),
        node_right: Array.prototype.slice.call(userAccountInfo.data.slice(299,331)),
        leaf_left: Array.prototype.slice.call(userAccountInfo.data.slice(331,363)),
        leaf_right: Array.prototype.slice.call(userAccountInfo.data.slice(363,395)),

    }
    return object;
}

const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';

const ADMIN_AUTH_KEY = new solana.PublicKey(new Uint8Array(constants.MERKLE_TREE_INIT_AUTHORITY));
const ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));
const MERKLE_TREE_KP = solana.Keypair.generate();
const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;
const MERKLE_TREE_SIZE = 16658;
export const DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};

describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();

  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;

  it("Initialize Merkle Tree", async () => {
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    // await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)

    const tx = await merkleTreeProgram.methods.initializeNewMerkleTree().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: MERKLE_TREE_KEY,
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
    var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
          MERKLE_TREE_KEY
        )

    assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
      merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
    )
    if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
      throw "merkle tree pda size wrong after initializing";

    }
    if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
      throw "merkle tree pda owner wrong after initializing";
    }
    // const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);

  });
  /*
  it("Init Merkle Tree Storage", async () => {
    const NODE_LEFT = Array(32).fill(2);
    const NODE_RIGHT = Array(32).fill(2);
    const ROOT_HASH = Array(32).fill(0);

    const INIT_STORAGE_DATA = []
          .concat(NODE_LEFT) // nullifier replacement
          .concat(NODE_LEFT)
          .concat(NODE_RIGHT)
          .concat(ROOT_HASH)
          .concat([...ADMIN_AUTH_KEY.toBytes()])
          .concat([...MERKLE_TREE_KEY.toBytes().values()])
    let pdas = getPdaAddresses({tx_integrity_hash: NODE_LEFT})

    const tx = await merkleTreeProgram.methods.initializeTmpMerkleTreeState(
      Buffer.from(INIT_STORAGE_DATA)
      ).accounts({
      authority: ADMIN_AUTH_KEY,
      verifierTmp: pdas.verifierStatePubkey,
      merkleTreeTmpStorage: pdas.merkleTreeTmpState,
      ...DEFAULT_PROGRAMS
    })
    .signers([ADMIN_AUTH_KEYPAIR])
    .rpc();
    var merkleTreeTmpAccountInfo = await provider.connection.getAccountInfo(
          pdas.merkleTreeTmpStoragePubkey
        )
    console.log("merkleTreeTmpAccountInfo: ", merkleTreeTmpAccountInfo.data.length)
    if (merkleTreeTmpAccountInfo.data.length !== constants.MERKLE_TREE_TMP_PDA_SIZE) {
      throw "merkle tree tmp pda size wrong after initializing";

    }
    if (merkleTreeTmpAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
      throw "merkle tree pda owner wrong after initializing";
    }

  });*/

  it("Groth16 verification hardcoded inputs should succeed", async () => {
    let userAccount =new anchor.web3.Account()
    await newAccountWithLamports(provider.connection, userAccount,verifierProgram ) // new anchor.web3.Account()
    // const newAccountWithLamports = async (connection,account = new anchor.web3.Account(),verifierProgram, lamports = 1e10) => {
    let init_account = await newAccountWithLamports(provider.connection ) // new anchor.web3.Account()

    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();

    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("prepare_inputs"),
          Buffer.from(userAccount.publicKey.toBytes()),
        ],
        verifierProgram.programId
      );

    let [pda1, bump1] = findProgramAddressSync(
        [
          Buffer.from(ix_data.txIntegrityHash),
          anchor.utils.bytes.utf8.encode("storage"),
        ],
        merkleTreeProgram.programId
      );

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1
    })
    console.log("ix_data.rootHash ", ix_data.rootHash)
    // pdas.verifierStatePubkey = pda;
    // pdas.merkleTreeTmpState = pda1;
    console.log("verifier_state: ", pdas.verifierStatePubkey.toBase58())
    console.log("merkleTreeTmpState: ", pdas.merkleTreeTmpState.toBase58())
    console.log("merkleTreeProgram.programId: ", merkleTreeProgram.programId.toBase58())
    await newAddressWithLamports(provider.connection, pdas.verifierStatePubkey) // new anchor.web3.Account()

    const tx = await verifierProgram.methods.createTmpAccount(
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
          // ix_data.merkleTreePdaPubkey,
          ix_data.encryptedUtxos,
          ix_data.merkleTreeIndex
          ).accounts(
              {
                signingAddress: init_account.publicKey,
                verifierState: pdas.verifierStatePubkey,
                systemProgram: SystemProgram.programId
              }
            ).signers([init_account]).rpc()

    await checkPreparedInputsAccountCreated({
      connection:provider.connection,
      pda: pdas.verifierStatePubkey,
      ix_data
    })
    var userAccountPublicKeyInfo = await provider.connection.getAccountInfo(
          userAccount.publicKey
        )
    try {
      let merkleTreeTmpStateDataBefore = await readAndParseAccountDataMerkleTreeTmpState({
        connection: provider.connection,
        pda: pdas.merkleTreeTmpState

      })
      console.log(merkleTreeTmpStateDataBefore)
    }catch{}

    const tx1 = await verifierProgram.methods.createMerkleTreeTmpAccount(
          ).accounts(
              {
                signingAddress: init_account.publicKey,
                verifierState: pdas.verifierStatePubkey,
                merkleTreeTmpState:pdas.merkleTreeTmpState,
                systemProgram: SystemProgram.programId,
                programMerkleTree: merkleTreeProgram.programId,
                rent: DEFAULT_PROGRAMS.rent,
              }
            ).signers([init_account]).rpc()
      var merkleTreeTmpAccountInfo = await provider.connection.getAccountInfo(
            pdas.merkleTreeTmpState
          )
      console.log("merkleTreeTmpAccountInfo: ", merkleTreeTmpAccountInfo.data.length)
      // if (merkleTreeTmpAccountInfo.data.length !== constants.MERKLE_TREE_TMP_PDA_SIZE) {
      //   throw "merkle tree tmp pda size wrong after initializing";
      //
      // }
      if (merkleTreeTmpAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
        throw "merkle tree pda owner wrong after initializing";
      }
      let merkleTreeTmpStateData = await readAndParseAccountDataMerkleTreeTmpState({
        connection: provider.connection,
        pda: pdas.merkleTreeTmpState

      })
      console.log(merkleTreeTmpStateData)
      // prepare inputs tx: 34
      await executeXComputeTransactions({
        number_of_transactions: PREPARED_INPUTS_TX_COUNT-2,
        userAccount: init_account,
        pdas: pdas,
        program: verifierProgram
      })

      await executeXComputeTransactions({
        number_of_transactions: MILLER_LOOP_TX_COUNT+2,
        userAccount,
        pdas: pdas,
        program: verifierProgram
      })
      await checkMillerLoopSuccess({
        connection:provider.connection,
        pda: pdas.verifierStatePubkey,
      })

      await executeXComputeTransactions({
        number_of_transactions: FINAL_EXPONENTIATION_TX_COUNT,
        userAccount,
        pdas: pdas,
        program: verifierProgram
      })

      await checkFinalExponentiationSuccess({
        connection:provider.connection,
        pda: pdas.verifierStatePubkey
      })
      try {
        await executeXComputeTransactionsMerkleTree({
          number_of_transactions: MERKLE_TREE_UPDATE_TX_COUNT,
          userAccount:    init_account,
          pdas:           pdas,
          program:        verifierProgram,
          number_of_instructions: true,
        })
      } catch(e){console.log(e)}

      let merkleTreeTmpStateDataAfter = await readAndParseAccountDataMerkleTreeTmpState({
        connection: provider.connection,
        pda: pdas.merkleTreeTmpState
      })
      console.log(merkleTreeTmpStateDataAfter)

  });
  /*

  it("Dynamic Shielded transaction", async () => {

    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

    let merkleTree = new MerkleTree(18);

    let inputUtxos = [new light.Utxo(), new light.Utxo()]
    let outputUtxos = [testOutputUtxo, new light.Utxo()]

    const data = await light.getProof(
      inputUtxos,
      outputUtxos,
      merkleTree,
      testOutputUtxo.amount.add(BigNumber.from(1)),
      U64(1),
      recipient,
      relayer,
      "deposit",
      encryptionKeypair
    )

    let ix_data = parse_instruction_data_bytes(data);

    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("prepare_inputs"),
          Buffer.from(ix_data.txIntegrityHash),
        ],
        verifierProgram.programId
      );


    const tx = await verifierProgram.methods.createTmpAccount(
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
          ix_data.merkleTreePdaPubkey,
          ix_data.encryptedUtxos,
          ix_data.merkleTreeIndex
          ).accounts(
              {
                signingAddress: userAccount.publicKey,
                verifierState: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount]).rpc()

      checkPreparedInputsAccountCreated({connection:provider.connection, pda, ix_data})
      // prepare inputs tx: 34
      await executeXComputeTransactions({number_of_transactions: PREPARED_INPUTS_TX_COUNT,userAccount,pda, verifierProgram})

      await executeXComputeTransactions({number_of_transactions: MILLER_LOOP_TX_COUNT,userAccount,pda, verifierProgram})

      await executeXComputeTransactions({number_of_transactions: FINAL_EXPONENTIATION_TX_COUNT,userAccount,pda, verifierProgram})
      await checkFinalExponentiationSuccess({connection:provider.connection, pda})

  });
*/

  async function executeXComputeTransactions({number_of_transactions,userAccount,pdas, program}) {
    let arr = []
    console.log(`sending ${number_of_transactions} transactions`)
    console.log(`verifierState ${pdas.verifierStatePubkey}`)
    console.log(`merkleTreeTmpState ${pdas.merkleTreeTmpState}`)

    for (var i = 0; i < number_of_transactions; i++) {

      let bump = new anchor.BN(i)
      const tx1 = await program.methods.compute(
              bump
            ).accounts(
                {
                  signingAddress: userAccount.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // verifierStateAuthority:pdas.verifierStatePubkey,
                  merkleTreeTmpState: pdas.merkleTreeTmpState,
                  programMerkleTree: merkleTreeProgram.programId,
                  merkleTree: MERKLE_TREE_KEY
                }
              ).signers([userAccount])
            .transaction();
        tx1.feePayer = userAccount.publicKey;
        // await userAccount.signTransaction(tx1);
        arr.push({tx:tx1, signers: [userAccount]})

      }
      await Promise.all(arr.map(async (tx, index) => {
      await provider.sendAndConfirm(tx.tx, tx.signers);
      }));
  }

  async function executeXComputeTransactionsMerkleTree({number_of_transactions,userAccount,pdas, program}) {
    let arr = []
    console.log(`sending ${number_of_transactions} transactions`)
    console.log(`verifierState ${pdas.verifierStatePubkey}`)
    console.log(`merkleTreeTmpState ${pdas.merkleTreeTmpState}`)
    let i = 0;
    let cache_index = 3;
    for(let ix_id = 0; ix_id < 38; ix_id ++) {
      let ix_data = [2, i];
      const transaction = new solana.Transaction();
      transaction.add(
        await program.methods.compute(new anchor.BN(i))
        .accounts({
          signingAddress: userAccount.publicKey,
          verifierState: pdas.verifierStatePubkey,
          // verifierStateAuthority:pdas.verifierStatePubkey,
          merkleTreeTmpState: pdas.merkleTreeTmpState,
          programMerkleTree: merkleTreeProgram.programId,
          merkleTree: MERKLE_TREE_KEY
        }).instruction()
      )
      if (ix_id != 0) {
        ix_data = [1, i];
        // const storageData = (await connection.getAccountInfo(MERKLE_TREE_TMP_STORAGE_KEY)).data;
        // const storage = STORAGE_LAYOUT.decode(storageData);
        // assert(+storage.currentInstructionIndex.toString() == cache_index, `CurrentInstructionIndex mismatch ${storage.currentInstructionIndex.toString()} <-> ${cache_index}`);
        // cache_index += 2;
        i+=1;
      }
      transaction.add(
        await program.methods.compute(new anchor.BN(i)).accounts({
          signingAddress: userAccount.publicKey,
          verifierState: pdas.verifierStatePubkey,
          // verifierStateAuthority:pdas.verifierStatePubkey,
          merkleTreeTmpState: pdas.merkleTreeTmpState,
          programMerkleTree: merkleTreeProgram.programId,
          merkleTree: MERKLE_TREE_KEY
        }).instruction()
      )
      arr.push({tx:transaction, signers: [userAccount]})
      // await program.provider.sendAndConfirm(transaction, [ADMIN_AUTH_KEYPAIR]);
    }/*
    for (var i = 0; i < number_of_transactions; i++) {

      let bump = new anchor.BN(i)
      const tx1 = new solana.Transaction();
      console.log("tx1: ", tx1)
      let instr = await program.methods.compute(
              bump
            ).accounts(
                {
                  signingAddress: userAccount.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // verifierStateAuthority:pdas.verifierStatePubkey,
                  merkleTreeTmpState: pdas.merkleTreeTmpState,
                  programMerkleTree: merkleTreeProgram.programId,
                  merkleTree: MERKLE_TREE_KEY
                }
              ).signers([userAccount])
            .instruction();
      console.log("instr: ", instr)
      tx1.add(instr);
      console.log("tx1: ", tx1)
      let bump1 = bump + 60
      tx1.add(await program.methods.compute(
            bump1
            ).accounts(
                {
                  signingAddress: userAccount.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // verifierStateAuthority:pdas.verifierStatePubkey,
                  merkleTreeTmpState: pdas.merkleTreeTmpState,
                  programMerkleTree: merkleTreeProgram.programId,
                  merkleTree: MERKLE_TREE_KEY
                }
              ).signers([userAccount])
            .instruction());
        console.log("tx1_2: ", tx1)
        tx1.feePayer = userAccount.publicKey;
        // await userAccount.signTransaction(tx1);
        arr.push({tx:tx1, signers: [userAccount]})

      }
    */
      console.log("arr: ", arr)
      await Promise.all(arr.map(async (tx, index) => {
      await provider.sendAndConfirm(tx.tx, tx.signers);
      }));
  }



  function getPdaAddresses({tx_integrity_hash, nullifier0, nullifier1}) {
    return {
      verifierStatePubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(tx_integrity_hash)), Buffer.from(constants.STORAGE_SEED)],
          verifierProgram.programId)[0],
      merkleTreeTmpState: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(tx_integrity_hash)), Buffer.from(constants.STORAGE_SEED)],
          merkleTreeProgram.programId)[0],
      leavesPdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier0)), Buffer.from(constants.LEAVES_SEED)],
          merkleTreeProgram.programId)[0],
      nullifier0PdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier0)), Buffer.from(constants.NF_SEED)],
          merkleTreeProgram.programId)[0],
      nullifier1PdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier1)), Buffer.from(constants.NF_SEED)],
          merkleTreeProgram.programId)[0],
    }
  }
  async function checkPreparedInputsAccountCreated({connection, pda, ix_data}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
    // assert_eq(accountAfterUpdate.proofAbc, ix_data.proofAbc, "proof insert wrong");
    assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    // assert_eq(accountAfterUpdate.signingAddress, ix_data.relayer, "relayer insert wrong");
    assert_eq(accountAfterUpdate.fee, ix_data.fee, "fee insert wrong");

    // if (accountAfterUpdate.merkleTreeTmpAccount.toBase58() != new solana.PublicKey(ix_data.merkleTreePdaPubkey).toBase58()) {
    //     throw ("merkleTreePdaPubkey insert wrong");
    // }
    assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");

  }

  async function checkMillerLoopSuccess({connection, pda}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
    const expectedMillerLoop = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];
    assert_eq(accountAfterUpdate.fBytes, expectedMillerLoop, "Miller loop failed");
  }

  async function checkFinalExponentiationSuccess({connection, pda}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
    const expectedFinalExponentiation = [13, 20, 220, 48, 182, 120, 53, 125, 152, 139, 62, 176, 232, 173, 161, 27, 199, 178, 181, 210,
      207, 12, 31, 226, 117, 34, 203, 42, 129, 155, 124, 4, 74, 96, 27, 217, 48, 42, 148, 168, 6,
      119, 169, 247, 46, 190, 170, 218, 19, 30, 155, 251, 163, 6, 33, 200, 240, 56, 181, 71, 190,
      185, 150, 46, 24, 32, 137, 116, 44, 29, 56, 132, 54, 119, 19, 144, 198, 175, 153, 55, 114, 156,
      57, 230, 65, 71, 70, 238, 86, 54, 196, 116, 29, 31, 34, 13, 244, 92, 128, 167, 205, 237, 90,
      214, 83, 188, 79, 139, 32, 28, 148, 5, 73, 24, 222, 225, 96, 225, 220, 144, 206, 160, 39, 212,
      236, 105, 224, 26, 109, 240, 248, 215, 57, 215, 145, 26, 166, 59, 107, 105, 35, 241, 12, 220,
      231, 99, 222, 16, 70, 254, 15, 145, 213, 144, 245, 245, 16, 57, 118, 17, 197, 122, 198, 218,
      172, 47, 146, 34, 216, 204, 49, 48, 229, 127, 153, 220, 210, 237, 236, 179, 225, 209, 27, 134,
      12, 13, 157, 100, 165, 221, 163, 15, 66, 184, 168, 229, 19, 201, 213, 152, 52, 134, 51, 44, 62,
      205, 18, 54, 25, 43, 152, 134, 102, 193, 88, 24, 131, 133, 89, 188, 39, 182, 165, 15, 73, 254,
      232, 143, 212, 58, 200, 141, 195, 231, 84, 25, 191, 212, 81, 55, 78, 37, 184, 196, 132, 91, 75,
      252, 189, 70, 10, 212, 139, 181, 80, 22, 228, 225, 237, 242, 147, 105, 106, 67, 183, 108, 138,
      95, 239, 254, 108, 253, 219, 89, 205, 123, 192, 36, 108, 23, 132, 6, 30, 211, 239, 242, 40, 10,
      116, 229, 111, 202, 188, 91, 147, 216, 77, 114, 225, 10, 10, 215, 128, 121, 176, 45, 6, 204,
      140, 58, 228, 53, 147, 108, 226, 232, 87, 34, 216, 43, 148, 128, 164, 111, 3, 153, 136, 168,
      12, 244, 202, 102, 156, 2, 97, 0, 248, 206, 63, 188, 82, 152, 24, 13, 236, 8, 210, 5, 93, 122,
      98, 26, 211, 204, 79, 221, 153, 36, 42, 134, 215, 200, 5, 40, 211, 180, 56, 196, 102, 146, 136,
      197, 107, 119, 171, 184, 54, 117, 40, 163, 31, 1, 197, 17];
      assert_eq(accountAfterUpdate.fBytes2, expectedFinalExponentiation, "Final Exponentiation failed");
  }

});
