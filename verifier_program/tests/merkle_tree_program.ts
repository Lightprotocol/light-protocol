import _ from "lodash"
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { MerkleTreeProgram, IDL } from "../target/types/merkle_tree_program";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { assert } from "chai";

import { struct, u8, u16, blob } from 'buffer-layout';
import { publicKey, u64, u128, } from '@project-serum/borsh';

export const DEFAULT_PROGRAMS = {
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: SYSVAR_RENT_PUBKEY,
  clock: SYSVAR_CLOCK_PUBKEY,
};

const constants:any = {};

const TYPE_PUBKEY = { array: [ 'u8', 32 ] };
const TYPE_SEED = {defined: "&[u8]"};
const TYPE_INIT_DATA = { array: [ 'u8', 642 ] };
const STORAGE_LAYOUT = struct([
  u8('isInitialized'),
  u8('accountType'),
  u64('currentInstructionIndex'),
  u8('foundRoot'),
  publicKey('relayer'),
  publicKey('merkleTreeKey'),
  publicKey('verifierKey'),
  publicKey('rootHash'),
  blob(96, 'state'),
  u64('currentRound'),
  u64('currentRoundIndex'),
  u64('currentIndex'),
  u64('currentLevel'),
  publicKey('currentRoundHash'),
  publicKey('nodeLeft'),
  publicKey('nodeRight'),
  publicKey('leafLeft'),
  publicKey('leafRight'),
]);

IDL.constants.map((item) => {
  if(_.isEqual(item.type, TYPE_SEED)) {
    constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
  } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
  {
    constants[item.name] = JSON.parse(item.value)
  }
});

describe("Merkle Tree Program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  const connection = program.provider.connection;

  const PRIVATE_KEY = [
    17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
    97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
    211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
    255, 166, 81,
  ];
  const ADMIN_AUTH_KEY = new PublicKey(new Uint8Array(constants.MERKLE_TREE_INIT_AUTHORITY));
  const ADMIN_AUTH_KEYPAIR = Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));

  const MERKLE_TREE_KEY_DEVNET = new PublicKey(new Uint8Array(constants.MERKLE_TREE_ACC_BYTES_0));
  const [MERKLE_TREE_KEY_PDA] = PublicKey.findProgramAddressSync([Buffer.from(constants.TREE_ROOT_SEED)], program.programId);
  const MERKLE_TREE_KP = Keypair.generate();
  // const merkleTreeKey = MERKLE_TREE_KEY_DEVNET;
  const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;

  const MERKLE_TREE_SIZE = 16658;

  const NODE_LEFT = Array(32).fill(2);
  const NODE_RIGHT = Array(32).fill(2);
  const ROOT_HASH = Array(32).fill(0);
  const VERIFIER_KEY = Array(32).fill(0);
  const INIT_STORAGE_DATA = []
        .concat(NODE_LEFT)
        .concat(NODE_RIGHT)
        .concat(ROOT_HASH)
        .concat([...ADMIN_AUTH_KEY.toBytes()])
        .concat([...MERKLE_TREE_KEY.toBytes().values()])
        .concat(VERIFIER_KEY)
  const CONCAT_DATA = Array(9).fill(0).concat(INIT_STORAGE_DATA);
  const [VERIFIER_TMP_STORAGE_KEY] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(73, 105)*/NODE_LEFT)), Buffer.from(constants.STORAGE_SEED)],
    program.programId
  );
  const [MERKLE_TREE_TMP_STORAGE_KEY] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(73, 105)*/NODE_LEFT)), Buffer.from(constants.STORAGE_SEED)],
    program.programId
  );
  const [TWO_LEAVES_KEY] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(105, 137)*/NODE_LEFT)), Buffer.from(constants.LEAVES_SEED)],
    program.programId
  );
  const [NF_KEY_1] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(105, 137)*/ADMIN_AUTH_KEY.toBytes())), Buffer.from(constants.NF_SEED)],
    program.programId
  );
  const [NF_KEY_2] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*IX_DATA.slice(137, 169)*/MERKLE_TREE_KEY.toBytes())), Buffer.from(constants.NF_SEED)],
    program.programId
  );
  it("Is initialized!", async () => {

    // const info = await connection.getAccountInfo(MERKLE_TREE_KEY_DEVNET);
    // console.log('merkle tree size cloned from devnet', info?.data.length, _.isEqual(MERKLE_TREE_SIZE, info?.data.length));

    console.log('Admin Key', ADMIN_AUTH_KEY.toString());

    const airdropTx = await connection.requestAirdrop(ADMIN_AUTH_KEY, 100_000_000_000_000);
    await connection.confirmTransaction(airdropTx);

    const programInfo = await connection.getAccountInfo(program.programId);
    console.log(programInfo.data);

  });
  it("Initialize Merkle Tree", async () => {

    const tx = await program.methods.initializeNewMerkleTree().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: MERKLE_TREE_KEY,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: MERKLE_TREE_KEY,
        space: MERKLE_TREE_SIZE,
        lamports: await connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: program.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
    .rpc();
  });

  it("Init Merkle Tree Storage", async () => {
    const tx = await program.methods.initializeTmpMerkleTree(Buffer.from(INIT_STORAGE_DATA)).accounts({
      authority: ADMIN_AUTH_KEY,
      verifierTmp: VERIFIER_TMP_STORAGE_KEY,
      merkleTreeTmpStorage: MERKLE_TREE_TMP_STORAGE_KEY,
      ...DEFAULT_PROGRAMS
    })
    .signers([ADMIN_AUTH_KEYPAIR])
    .rpc();
  });
  it("Update Merkle Tree", async () => {
    let i = 0;
    let cache_index = 3;
    for(let ix_id = 0; ix_id < 38; ix_id ++) {
      let ix_data = [2, i];
      const transaction = new Transaction();
      transaction.add(
        await program.methods.updateMerkleTree(Buffer.from(ix_data)).accounts({
          authority: ADMIN_AUTH_KEY,
          merkleTreeTmpStorage: MERKLE_TREE_TMP_STORAGE_KEY,
        }).remainingAccounts([{
          isSigner: false,
          isWritable: true,
          pubkey: MERKLE_TREE_KEY,
        }])
        .instruction()
      )
      if (ix_id != 0) {
        ix_data = [1, i];
        const storageData = (await connection.getAccountInfo(MERKLE_TREE_TMP_STORAGE_KEY)).data;
        const storage = STORAGE_LAYOUT.decode(storageData);
        assert(+storage.currentInstructionIndex.toString() == cache_index, `CurrentInstructionIndex mismatch ${storage.currentInstructionIndex.toString()} <-> ${cache_index}`);
        cache_index += 2;
      }
      transaction.add(
        await program.methods.updateMerkleTree(Buffer.from(ix_data)).accounts({
          authority: ADMIN_AUTH_KEY,
          merkleTreeTmpStorage: MERKLE_TREE_TMP_STORAGE_KEY,
        }).remainingAccounts([{
          isSigner: false,
          isWritable: true,
          pubkey: MERKLE_TREE_KEY,
        }])
        .instruction()
      )
      await program.provider.sendAndConfirm(transaction, [ADMIN_AUTH_KEYPAIR]);
    }
  });


});
