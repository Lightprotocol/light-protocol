const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import * as anchor from "@project-serum/anchor";
import fs from 'fs';
export const read_and_parse_instruction_data_bytes = ()  => {
  let file = fs.readFileSync('tests/deposit.txt','utf8');

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

export function parse_instruction_data_bytes(data) {
   let ix_data = {
     rootHash:          data.data.publicInputsBytes.slice(0,32),
     amount:             data.data.publicInputsBytes.slice(32,64),
     txIntegrityHash:  data.data.publicInputsBytes.slice(64,96),
     nullifier0:         data.data.publicInputsBytes.slice(96,128),
     nullifier1:         data.data.publicInputsBytes.slice(128,160),
     leafRight:         data.data.publicInputsBytes.slice(160,192),
     leafLeft:          data.data.publicInputsBytes.slice(192,224),
     proofAbc:        data.data.proofBytes,
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

export async function readAndParseAccountDataMerkleTreeTmpState({
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

export async function getPdaAddresses({tx_integrity_hash,
  nullifier0, nullifier1, leafLeft,
  merkleTreeProgram, verifierProgram
}) {
  console.log("new Uint8Array(nullifier0) ", new Uint8Array(nullifier0));

  return {
    signerAuthorityPubkey: (await solana.PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBytes()],
        verifierProgram.programId))[0],
    verifierStatePubkey: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("storage")],
        verifierProgram.programId))[0],
    feeEscrowStatePubkey: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("escrow")],
        verifierProgram.programId))[0],
    merkleTreeUpdateState: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(leafLeft)), anchor.utils.bytes.utf8.encode("storage")],
        merkleTreeProgram.programId))[0],
    leavesPdaPubkey: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(nullifier0)), anchor.utils.bytes.utf8.encode("leaves")],
        merkleTreeProgram.programId))[0],
    nullifier0PdaPubkey: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(nullifier0)), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId))[0],
    nullifier1PdaPubkey: (await solana.PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(nullifier1)), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId))[0]
  }
}

export function unpackLeavesAccount(leavesAccountData) {
  return{
    leafType: leavesAccountData[1],
    leafIndex:    U64.readLE(leavesAccountData.slice(2,10),0),
    leafLeft:     Array.prototype.slice.call(leavesAccountData.slice(10, 42)),
    leafRight:    Array.prototype.slice.call(leavesAccountData.slice(42, 74)),
    encryptedUtxos: Array.prototype.slice.call(leavesAccountData.slice(106,328 + 16)),
    merkleTree:   Array.prototype.slice.call(leavesAccountData.slice(74, 106)),
  }
}
