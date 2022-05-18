import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");

const newAccountWithLamports = async (connection, lamports = 1e10) => {
  const account = new anchor.web3.Account()

  let retries = 30
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
  for (var i = 0; i < value0; i++) {
    if (value0[i] !== value1[i]) {
      throw Error(message);
    }
  }

}

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

describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();

  const program = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  /*
  it("Is initialized!", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("data_holder_v0"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();

    while (ix_data.encryptedUtxos.length < 256) {
      ix_data.encryptedUtxos.push(0);
    }
    const tx = await program.methods.createTmpAccount(
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
                prepareInputsState: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount])
            .rpc();

    const userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = program.account.prepareInputsState._coder.accounts.decode('PrepareInputsState', userAccountInfo.data);
    // console.log(accountAfterUpdate)
    // const accountAfterUpdate = await program.account.prepareInputsState.fetch(pda);
    console.log(accountAfterUpdate);

    assert_eq(accountAfterUpdate.proofAbc, ix_data.proofAbc, "proof insert wrong");
    assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    // assert_eq(accountAfterUpdate.signingAddress, ix_data.relayer, "relayer insert wrong");
    assert_eq(accountAfterUpdate.fee, ix_data.fee, "fee insert wrong");

    if (accountAfterUpdate.merkleTreeTmpAccount.toBase58() != new solana.PublicKey(ix_data.merkleTreePdaPubkey).toBase58()) {
        throw ("merkleTreePdaPubkey insert wrong");
    }
    assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");
    // assert_eq(accountAfterUpdate.nullifier0, ix_data.nullifier0, "nullifier0 insert wrong");
    // assert_eq(accountAfterUpdate.nullifier1, ix_data.nullifier1, "nullifier1 insert wrong");
    // assert_eq(accountAfterUpdate.leafRight, ix_data.leafRight, "leafRight insert wrong");
    // assert_eq(accountAfterUpdate.leafLeft, ix_data.leafLeft, "leafLeft insert wrong");
    // assert_eq(accountAfterUpdate.recipient, ix_data.recipient, "recipient insert wrong");
    // assert_eq(accountAfterUpdate.encryptedUtxos, ix_data.encryptedUtxos, "encryptedUtxos insert wrong");

    // console.log("program.accoun: ", program.account.prepareInputsState)

    // console.log(userAccountInfo.data.slice(0,32))
  });
  */
  it("Prepared inputs", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("data_holder_v0"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();


    while (ix_data.encryptedUtxos.length < 256) {
      ix_data.encryptedUtxos.push(0);
    }
    const tx = await program.methods.createTmpAccount(
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
                prepareInputsState: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount]).rpc()
      // requestHeapFrame
      for (var i = 0; i < 464; i++) {
        console.log("tx: ", i)
        const tx1 = await program.methods.prepareInputs(
              ).accounts(
                  {
                    signingAddress: userAccount.publicKey,
                    prepareInputsState: pda,
                  }
                ).signers([userAccount])
                .rpc();
        var userAccountInfoI = await provider.connection.getAccountInfo(
              pda
            )
            const accountAfterUpdateI = program.account.prepareInputsState._coder.accounts.decode('PrepareInputsState', userAccountInfoI.data);
            console.log("accountAfterUpdateI.resXRange ", accountAfterUpdateI.resXRange)
            console.log("accountAfterUpdateI.resYRange ", accountAfterUpdateI.resYRange)
            console.log("accountAfterUpdateI.resZRange ", accountAfterUpdateI.resZRange)
            console.log("current_instruction_index ", accountAfterUpdateI.current_instruction_index)
      }

    // console.log("Your transaction signature", tx);
    // const accountInfo = await program.getAccountInfo( new solana.PublicKey(storage_account_pkey) );
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = program.account.prepareInputsState._coder.accounts.decode('PrepareInputsState', userAccountInfo.data);
    console.log(accountAfterUpdate)
    // const accountAfterUpdate = await program.account.prepareInputsState.fetch(pda);
    // console.console.log(accountAfterUpdate);

    assert_eq(accountAfterUpdate.proofAbc, ix_data.proofAbc, "proof insert wrong");
    assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    // assert_eq(accountAfterUpdate.signingAddress, ix_data.relayer, "relayer insert wrong");
    assert_eq(accountAfterUpdate.fee, ix_data.fee, "fee insert wrong");

    if (accountAfterUpdate.merkleTreeTmpAccount.toBase58() != new solana.PublicKey(ix_data.merkleTreePdaPubkey).toBase58()) {
        throw ("merkleTreePdaPubkey insert wrong");
    }
    assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");
    // assert_eq(accountAfterUpdate.nullifier0, ix_data.nullifier0, "nullifier0 insert wrong");
    // assert_eq(accountAfterUpdate.nullifier1, ix_data.nullifier1, "nullifier1 insert wrong");
    // assert_eq(accountAfterUpdate.leafRight, ix_data.leafRight, "leafRight insert wrong");
    // assert_eq(accountAfterUpdate.leafLeft, ix_data.leafLeft, "leafLeft insert wrong");
    // assert_eq(accountAfterUpdate.recipient, ix_data.recipient, "recipient insert wrong");
    // assert_eq(accountAfterUpdate.encryptedUtxos, ix_data.encryptedUtxos, "encryptedUtxos insert wrong");

    // console.log("program.accoun: ", program.account.prepareInputsState)

    // console.log(userAccountInfo.data.slice(0,32))
  });

});
