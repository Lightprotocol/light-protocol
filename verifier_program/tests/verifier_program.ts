import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");

const PREPARED_INPUTS_TX_COUNT = 35
const MILLER_LOOP_TX_COUNT = 41
const FINAL_EXPONENTIATION_TX_COUNT = 17
const MERKLE_TREE_UPDATE_TX_COUNT = 0

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
  for (var i = 0; i < value0.length; i++) {
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
                VerifierState: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount])
            .rpc();

    const userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = program.account.VerifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
    // console.log(accountAfterUpdate)
    // const accountAfterUpdate = await program.account.VerifierState.fetch(pda);
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

    // console.log("program.accoun: ", program.account.VerifierState)

    // console.log(userAccountInfo.data.slice(0,32))
  });
*/
  it("Prepared inputs", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let x = Buffer.from(ix_data.txIntegrityHash);
    console.log(x)

    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("prepare_inputs"),
          x,
        ],
        program.programId
      );
    console.log(pda.toBase58())

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
                verifierState: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount]).rpc()

      checkPreparedInputsAccountCreated({connection:provider.connection, pda, ix_data})
      // prepare inputs tx: 34
      await executeXTransactions({number_of_transactions: PREPARED_INPUTS_TX_COUNT +1,userAccount,pda, program})


      await executeXTransactions({number_of_transactions: MILLER_LOOP_TX_COUNT,userAccount,pda, program})
      await checkMillerLoopSuccess({connection:provider.connection, pda})
      // await executeXTransactions({number_of_transactions: MILLER_LOOP_TX_COUNT,userAccount,pda, program})

  });

  async function executeXTransactions({number_of_transactions,userAccount,pda, program}) {
    let arr = []
    console.log(`sending ${number_of_transactions} transactions`)
    for (var i = 0; i < number_of_transactions; i++) {

      let bump = new anchor.BN(i)
      const tx1 = await program.methods.compute(
              bump
            ).accounts(
                {
                  signingAddress: userAccount.publicKey,
                  verifierState: pda,
                }
              ).signers([userAccount])
            .transaction();
        tx1.feePayer = userAccount.publicKey;
        // await userAccount.signTransaction(tx1);
        arr.push({tx:tx1, signers: [userAccount]})

      }
      //   console.log(program.provider)
      // await promise.all()
      // await provider.sendAll(arr);
      await Promise.all(arr.map(async (tx, index) => {
      await provider.sendAndConfirm(tx.tx, tx.signers);
      }));
  }

  async function checkPreparedInputsAccountCreated({connection, pda, ix_data}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = program.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
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

  }

  async function checkMillerLoopSuccess({connection, pda}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    const accountAfterUpdate = program.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
    const expectedMillerLoop = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];
    assert_eq(accountAfterUpdate.fBytes, expectedMillerLoop, "Miller loop failed");
  }
  /*
  it("Miller Loop", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let [pda_prepare_inputs, bump_0] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("prepare_inputs"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let [pda_miller_loop, bump_1] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("miller_loop"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    ix_data.prepared_inputs_bytes = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];

    const tx = await program.methods.createMillerLoopAccount(
          ix_data.proofAbc,
          ).accounts(
              {
                signingAddress: userAccount.publicKey,
                millerLoopState: pda_miller_loop,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount]).rpc()

      var userAccountInfo1 = await provider.connection.getAccountInfo(
            pda_miller_loop
          )
      const accountAfterUpdate1 = program.account.millerLoopState._coder.accounts.decode('MillerLoopState', userAccountInfo1.data);
      console.log(accountAfterUpdate1)
      let arr = []
      console.log("creating 42 txs")

      for (var i = 0; i < 42; i++) {
        // let signer_2 = solana.Keypair.generate();
        const tx1 = await program.methods.computeMillerLoop(i
              ).accounts(
                  {
                    signingAddress: userAccount.publicKey,
                    millerLoopState: pda_miller_loop,
                  }
                ).signers([userAccount])
                .transaction();
        tx1.feePayer = userAccount.publicKey;
        // await userAccount.signTransaction(tx1);
        arr.push({tx:tx1, signers: [userAccount]})

      }
    //   console.log(program.provider)
    // await promise.all()
    // await provider.sendAll(arr);
  await Promise.all(arr.map(async (tx, index) => {
    await provider.sendAndConfirm(tx.tx, tx.signers);
  }));

    // console.log("Your transaction signature", tx);
    // const accountInfo = await program.getAccountInfo( new solana.PublicKey(storage_account_pkey) );
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda_miller_loop
        )
    const accountAfterUpdate = program.account.millerLoopState._coder.accounts.decode('MillerLoopState', userAccountInfo.data);
    console.log(accountAfterUpdate)
    // const accountAfterUpdate = await program.account.VerifierState.fetch(pda);
    // console.console.log(accountAfterUpdate);

    assert_eq(accountAfterUpdate.preparedInputsBytes, ix_data.prepared_inputs_bytes, "preparedInputsBytes insert wrong");
    // assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    // assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    // assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    // assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    // // assert_eq(accountAfterUpdate.signingAddress, ix_data.relayer, "relayer insert wrong");
    // assert_eq(accountAfterUpdate.fee, ix_data.fee, "fee insert wrong");
    //
    // if (accountAfterUpdate.merkleTreeTmpAccount.toBase58() != new solana.PublicKey(ix_data.merkleTreePdaPubkey).toBase58()) {
    //     throw ("merkleTreePdaPubkey insert wrong");
    // }
    // assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");

  });

  it("Final Exponentiation", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let [pda_prepare_inputs, bump_0] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("prepare_inputs"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let [pda_miller_loop, bump_1] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("miller_loop"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );

    let [pda_final_exponentiation, bump_2] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("final_exponentiation"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );


    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    ix_data.prepared_inputs_bytes = [220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89, 0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187, 136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133, 250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15];

    const tx = await program.methods.createFinalExponentiationAccount(
          ).accounts(
              {
                signingAddress: userAccount.publicKey,
                finalExponentiationState: pda_final_exponentiation,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount]).rpc()

      var userAccountInfo1 = await provider.connection.getAccountInfo(
            pda_final_exponentiation
          )
      const accountAfterUpdate1 = program.account.millerLoopState._coder.accounts.decode('FinalExponentiationState', userAccountInfo1.data);
      console.log(accountAfterUpdate1)
      let arr = []
      console.log("creating 42 txs")
      console.log(program.methods)
      for (var i = 0; i < 18; i++) {
        // let signer_2 = solana.Keypair.generate();
        let bump = new anchor.BN(i)
        const tx1 = await program.methods.computeFinalExponetiation(
                bump
              ).accounts(
                  {
                    signingAddress: userAccount.publicKey,
                    finalExponentiationState: pda_final_exponentiation,
                  }
                ).signers([userAccount])
                .transaction();
        tx1.feePayer = userAccount.publicKey;
        // tx1.add(solana.ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }));

        // await userAccount.signTransaction(tx1);
        arr.push({tx:tx1, signers: [userAccount]})

      }
    //   console.log(program.provider)
    // await promise.all()
    // await provider.sendAll(arr);
    console.log(arr[0].tx.instructions[0])
    console.log(arr[0].tx.instructions[1])

  await Promise.all(arr.map(async (tx, index) => {
    await provider.sendAndConfirm(tx.tx, tx.signers);
  }));

    // console.log("Your transaction signature", tx);
    // const accountInfo = await program.getAccountInfo( new solana.PublicKey(storage_account_pkey) );
    var userAccountInfo = await provider.connection.getAccountInfo(
          pda_final_exponentiation
        )
    // const accountAfterUpdate = program.account.millerLoopState._coder.accounts.decode('FinalExponentiationState', userAccountInfo.data);
    // console.log(accountAfterUpdate)
    // // const accountAfterUpdate = await program.account.VerifierState.fetch(pda);
    // // console.console.log(accountAfterUpdate);
    //
    // assert_eq(accountAfterUpdate.preparedInputsBytes, ix_data.prepared_inputs_bytes, "preparedInputsBytes insert wrong");
    // assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    // assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    // assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    // assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    // // assert_eq(accountAfterUpdate.signingAddress, ix_data.relayer, "relayer insert wrong");
    // assert_eq(accountAfterUpdate.fee, ix_data.fee, "fee insert wrong");
    //
    // if (accountAfterUpdate.merkleTreeTmpAccount.toBase58() != new solana.PublicKey(ix_data.merkleTreePdaPubkey).toBase58()) {
    //     throw ("merkleTreePdaPubkey insert wrong");
    // }
    // assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");

  });
  */
});
