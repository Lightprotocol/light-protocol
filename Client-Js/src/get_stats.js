#!/usr/bin/env node

const solana = require('@solana/web3.js')
const solanaRPC = 'https://api.devnet.solana.com';//'http://localhost:8899'//

var rust = require("./webassembly");



const { bigInt } = require('snarkjs')

const crypto = require('crypto')

const program = require('commander');
const { UTF8 } = require('buffer-layout');


//const merkle_tree_storage_acc_pkey = 'CaxjyUvDcRMozmaMa1Bg9TeakyW9jskXx3b7DhNCFvTX';
//const merkle_tree_storage_acc_pkey = '9XUqBBMR2JTTADpCs33U4FhsZLKSyev8oiqZAmnxAz3X'
//const merkle_tree_storage_acc_pkey = '54zcGRPBRmRq38d5cmFg9rD9ixDzzwPW8J5NmJfECugQ'
const merkle_tree_storage_acc_pkey = 'Hk4jivMBjLQBEdap6rntYcWUZub5tZ25uLPW2VogTkQK'


let acc =process.env.PRIVATE_KEY

async function getLeaves(pubkey) {
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(pubkey) )
  // const data = accountInfo.data
  const data = [...accountInfo.data] // read from buffer
  // console.log("acc:  ", accountInfo)
  // let leaves = []
  let min_range = 3937 // hc based on merkle_tree struct fields in program/lib.rs
  let max_range = 69473 // hc no. of leaves
  //console.log("Tree height: ", rust.bytes_to_int(data.slice(1, 9)))
  //check that tree height is compatible
  if (rust.bytes_to_int(data.slice(1, 9)) != 11) {
        throw "Tree height is not 11";
    }
  //console.log("NEXTIDX:  ", rust.bytes_to_int(data.slice(721, 729)))
  let next_index_bigInt = rust.bytes_to_int(data.slice(721, 729)) // hc, usize
  let next_index = Number(next_index_bigInt)
  console.log("nextIndex aftr rust: ",next_index)
  console.log()
  let leaves_bytes_range = data.slice(min_range,min_range + (next_index)*32 ) // get only filled leaves. nextIndex seems to be "currentLeaves"
  //console.log("filled leaves bytes: ", leaves_bytes_range, "compared: ", data.slice(min_range, max_range))

  //console.log("tree root : ", data.slice(min_range-3200, min_range));
  // create 32 byte chunks inside the range of leaves
  // let chunkSize = 32 // s: size of chunks
  // const leaves = [];
  // for (let i = 0; i < leaves_bytes_range.length; i += chunkSize) {
  //     const chunk = leaves_bytes_range.slice(i, i + chunkSize);
  //     leaves.push(chunk);
  // }


  return new Uint8Array(leaves_bytes_range) // unchunked
}

function createDeposit({ nullifier, secret }) {

  const deposit = {nullifier, secret} // , preimage, commitment, commitmentHex, nullifierHash, nullifierHex
  deposit.preimage = Buffer.concat([deposit.nullifier, deposit.secret])
  deposit.commitment = rust.hash_slice_u8(deposit.preimage);  // DEPOSIT: INSERT INTO MERKLE TREE EXPECTS HEX string but found: arr (changed currently)
  deposit.commitmentHex = toHex(Buffer.from(deposit.commitment))
  deposit.nullifierHash = rust.hash_slice_u8(deposit.nullifier);
  deposit.nullifierHex = toHex(Buffer.from(deposit.nullifierHash))
  console.log("deposit::", deposit)
  return deposit
}


function parseNote(noteString) { // withdrawal takes note (unhashed hex)
  const noteRegex = /light-(?<currency>\w+)-(?<amount>[\d.]+)-0x(?<note>[0-9a-fA-F]{128})/g
  const match = noteRegex.exec(noteString)
  if (!match) {
    throw new Error('The note has invalid format')
  }

  const buf = Buffer.from(match.groups.note, 'hex')
  const nullifier = buf.slice(0, 32)
  const secret = buf.slice(32, 64)
  console.log(secret);
  const deposit = createDeposit({ nullifier, secret })
  // const netId = Number(match.groups.netId)

  return {  deposit }
}
async function readAcc_Miller(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let init_data = data.slice(0,220)
  let tx_integrity_hash = data.slice(190,212)
  let f_r = data.slice(220,576 + 220)
  let coeff2_r = data.slice(1728+ 220,1824+ 220)
  let coeff1_r = data.slice(1824+ 220,1920+ 220)
  let coeff0_r = data.slice(2208+ 220,2304 + 220)
  let r_r = data.slice(3360+ 220,3648+ 220)
  let proof_b_r = data.slice(3648+ 220,3840+ 220)
  let prepared_inputs = data.slice(2908,3004)

  let v = "";
  // let counter = 0;
  for(const item of init_data){
    v += item + " ";
    // counter += 1;
  }
  console.log("init_data: ", v);
  v = "";
  // let counter = 0;
  for(const item of prepared_inputs){
    v += item + " ";
    // counter += 1;
  }
  console.log("prepared_inputs: ", v);

  v = "";
  // let counter = 0;
  for(const item of f_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("f_r: ", v);

  v = "";
  // let counter = 0;
  for(const item of tx_integrity_hash){
    v += item + " ";
    // counter += 1;
  }
  console.log("------------------------------------------");

  console.log("tx_integrity_hash: ", v);


  v = "";
  // let counter = 0;
  for(const item of coeff2_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff2: ",v);

  v = "";
  // let counter = 0;
  for(const item of coeff1_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff1: ",v);

  v = "";
  // let counter = 0;
  for(const item of coeff0_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff0: ",v);

  v = "";
  // let counter = 0;
  for(const item of r_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("r: ",v);

  v = "";
  // let counter = 0;
  for(const item of proof_b_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("proofb: ",v);
}

async function readAcc(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let v = "";
  let counter = 0;
  for(const item of data){
    if (counter == 3841) {
      break
    }
    v += item + " ";
    counter += 1;
  }
  console.log(v);
  //console.log(data);
}

async function readAccMerkletreeRoots(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let v = "";
  let counter = 0;
  for(const item of data){
    if (counter > 737 && counter <1000) {
      v += item + " ";
      counter += 1;
    } else if (counter > 1000) {
      break;
    }
    counter += 1;
  }
  console.log(v);
  //console.log(data);
}

async function readAccMerkletreeRoots(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let v = "";
  let counter = 0;
  for(const item of data){
    if (counter > 737 && counter <1000) {
      v += item + " ";
      counter += 1;
    } else if (counter > 1000) {
      break;
    }
    counter += 1;
  }
  console.log(v);
  //console.log(data);
}


async function readAccMerkletreeNullifier(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let v = "";
  let counter = 0;
  for(const item of data){
    if (counter > 69481 && counter < 69801) {
      v += item + " ";
      counter += 1;
    } else if (counter > 69801) {
      break;
    }
    counter += 1;
  }
  console.log(v);
  //console.log(data);
}

async function readAcc_Miller(account_pkey){
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(account_pkey) )
  const data = Buffer.from(accountInfo.data)
  //console.dir(data, { depth: null });
  let init_data = data.slice(0,220)
  let tx_integrity_hash = data.slice(190,212)
  let f_r = data.slice(220,576 + 220)
  let coeff2_r = data.slice(1728+ 220,1824+ 220)
  let coeff1_r = data.slice(1824+ 220,1920+ 220)
  let coeff0_r = data.slice(2208+ 220,2304 + 220)
  let r_r = data.slice(3360+ 220,3648+ 220)
  let proof_b_r = data.slice(3648+ 220,3840+ 220)
  let prepared_inputs = data.slice(2908,3004)

  let v = "";
  // let counter = 0;
  for(const item of init_data){
    v += item + " ";
    // counter += 1;
  }
  console.log("init_data: ", v);
  v = "";
  // let counter = 0;
  for(const item of prepared_inputs){
    v += item + " ";
    // counter += 1;
  }
  console.log("prepared_inputs: ", v);

  v = "";
  // let counter = 0;
  for(const item of f_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("f_r: ", v);

  v = "";
  // let counter = 0;
  for(const item of tx_integrity_hash){
    v += item + " ";
    // counter += 1;
  }
  console.log("------------------------------------------");

  console.log("tx_integrity_hash: ", v);


  v = "";
  // let counter = 0;
  for(const item of coeff2_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff2: ",v);

  v = "";
  // let counter = 0;
  for(const item of coeff1_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff1: ",v);

  v = "";
  // let counter = 0;
  for(const item of coeff0_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("coeff0: ",v);

  v = "";
  // let counter = 0;
  for(const item of r_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("r: ",v);

  v = "";
  // let counter = 0;
  for(const item of proof_b_r){
    v += item + " ";
    // counter += 1;
  }
  console.log("proofb: ",v);
}

async function check_deposit(noteString, testmode) {
  let parsed_note = parseNote(noteString)
  console.log("parsed_note", parsed_note)
  //await readAccStats(merkle_tree_storage_acc_pkey);
  //checks
  let leaves = await getLeaves(merkle_tree_storage_acc_pkey);
  console.log(leaves)
  //leaf has been inserted
  let chunkSize = 32 // s: size of chunks
  let found_leaf = 0
  let leaf_position
  for (let i = 0; i < leaves.length; i += chunkSize) {
      const chunk = leaves.slice(i, i + chunkSize);
      console.log(` ${chunk}`)
      //console.log(parsed_note.deposit.commitment.slice(0,32) == chunk)
      if ((parsed_note.deposit.commitment).toString() == (chunk).toString() ){
        found_leaf+=1;
        //console.log("found_leaf at position " + i /32)
        console.log("next leaf ", leaves.slice(i +32, i + chunkSize +32))
        console.log("current leaf ", leaves.slice(i, i + chunkSize ))

        console.log("previous leaf ", leaves.slice(i -32, i + chunkSize -32))

        leaf_position = i /32
      }
  }

  console.assert(found_leaf == true, "Leaf not inserted")

  //nullifier exists
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(merkle_tree_storage_acc_pkey) )
  // const data = accountInfo.data
  const data = [...accountInfo.data] // read from buffer
  // console.log("acc:  ", accountInfo)
  // let leaves = []
  let min_range = 69481 // hc based on merkle_tree struct fields in program/lib.rs
  //let max_range = 69473 // hc no. of leaves
  let nullifiers = data.slice(min_range, data.length)

  let found_nullifier = false
  for (let i = 0; i < nullifiers.length; i += chunkSize) {
      const chunk = nullifiers.slice(i, i + chunkSize);

      //console.log(`${parsed_note.deposit.nullifierHash} == ${chunk}`)
      if(parsed_note.deposit.nullifierHash.toString() == chunk.toString()) {
        found_nullifier = true
        console.log(found_nullifier)

      }
  }
  console.assert(found_nullifier == false, "Nullifier found")
  let is_last_leaf = false
  if (testmode == true) {
    //check leaf index == last
    //console.log(`${leaves.slice(leaves.length - 32, leaves.length).toString()} == ${parsed_note.deposit.commitment.toString()}`)
    if(leaves.slice(leaves.length - 32, leaves.length).toString() == parsed_note.deposit.commitment.toString()){
      is_last_leaf = true

    }
  }
  return {found_leaf: found_leaf, found_nullifier: found_nullifier, is_last_leaf: is_last_leaf, leaf_position: leaf_position}
}

async function check_merkle_tree_status() {
  //await readAccStats(merkle_tree_storage_acc_pkey);
  //checks
  let leaves = await getLeaves(merkle_tree_storage_acc_pkey);
  console.log(leaves)
  //leaf has been inserted
  let chunkSize = 32 // s: size of chunks
  let found_leaf = 0
  let leaf_position
  console.log("Number of leaves: ", leaves.length / 32);


  //console.assert(found_leaf == true, "Leaf not inserted")

  //nullifier exists
  const accountInfo = await connection.getAccountInfo( new solana.PublicKey(merkle_tree_storage_acc_pkey) )
  // const data = accountInfo.data
  const data = [...accountInfo.data] // read from buffer
  // console.log("acc:  ", accountInfo)
  // let leaves = []
  let min_range = 69481 // hc based on merkle_tree struct fields in program/lib.rs
  //let max_range = 69473 // hc no. of leaves
  let nullifiers = data.slice(min_range, data.length)

  let found_nullifier = false
  for (let i = 0; i < nullifiers.length; i += chunkSize) {
      const chunk = nullifiers.slice(i, i + chunkSize);

      //console.log(`${parsed_note.deposit.nullifierHash} == ${chunk}`)
      if((new Uint8Array(32).fill(0)).toString() == chunk.toString()) {
        found_nullifier = true
        console.log("Number of nullifers: ", i /32);

        break
      }
  }
}

// Connection check
async function getNodeConnection(url) {
  connection = new solana.Connection(url, 'recent')
  const version = await connection.getVersion()
  //console.log('Connection to cluster established:', url, version)
}
const toHex = (number, length = 32) => '0x' + (number instanceof Buffer ? number.toString('hex') : "") // buffer has own implementation of that...


async function main(){
  // if in cli
  program
    .option('-r, --rpc <URL>', 'The RPC, CLI should interact with', 'http://localhost:8899')
    .option('-t <BOOL>', 'test')

  program
        .command('check_deposit <note>')
        .description('Show all merkletree roots saved in the account onchain for testing.')
        .action(
          async(note) => getNodeConnection(solanaRPC).then(async function(){
            //await readAccMerkletreeRoots(merkle_tree_storage_acc_pkey); //:ok smth
            //let pubkey = new solana.PublicKey(merkle_tree_storage_acc_pkey);
            console.log( await check_deposit(note,true))
            //console.log(Uint8Array.from(new solana.PublicKey(merkle_tree_storage_acc_pkey).toBuffer()))
            //console.log("[251, 30, 194, 174, 168, 85, 13, 188, 134, 0, 17, 157, 187, 32, 113, 104, 134, 138, 82, 128, 95, 206, 76, 34, 177, 163, 246, 27, 109, 207, 2, 85]")

          })
        )
  program
        .command('check_merkle_tree')
        .description('Show all merkletree roots saved in the account onchain for testing.')
        .action(
          async(note) => getNodeConnection(solanaRPC).then(async function(){
            //await readAccMerkletreeRoots(merkle_tree_storage_acc_pkey); //:ok smth
            //let pubkey = new solana.PublicKey(merkle_tree_storage_acc_pkey);
             await check_merkle_tree_status()
            //console.log(Uint8Array.from(new solana.PublicKey(merkle_tree_storage_acc_pkey).toBuffer()))
            //console.log("[251, 30, 194, 174, 168, 85, 13, 188, 134, 0, 17, 157, 187, 32, 113, 104, 134, 138, 82, 128, 95, 206, 76, 34, 177, 163, 246, 27, 109, 207, 2, 85]")

          })
        )

  program.parse(process.argv);

}


main()
