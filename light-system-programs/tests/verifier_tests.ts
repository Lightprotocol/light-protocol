import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import _ from "lodash";
import { assert } from "chai";
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

import {
  Transaction,
  VerifierZero,
  VerifierOne,
  Account,
  Utxo,
  createMintWrapper,
  setUpMerkleTree,
  initLookUpTableFromFile,
  MerkleTreeProgram,
  merkleTreeProgramId,
  IDL_MERKLE_TREE_PROGRAM,
  MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  KEYPAIR_PRIVKEY,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_ONE_PDA,
  USER_TOKEN_ACCOUNT,
  createTestAccounts,
  userTokenAccount,
  recipientTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  Provider as LightProvider,
  Relayer,
  checkNfInserted,
  newAccountWithTokens,
  Action,
  useWallet,
  TestRelayer
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";

var LOOK_UP_TABLE, POSEIDON, KEYPAIR,RELAYER, deposit_utxo1 ;

var transactions: Transaction[] = [];
console.log = () => {};
describe("Verifier Zero and One Tests", () => {
 // Configure the client to use the local cluster.
 process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

 const provider = anchor.AnchorProvider.local(
   "http://127.0.0.1:8899",
   confirmConfig
 );
 anchor.setProvider(provider);
 process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
//  console.log = () => {};
 const merkleTreeProgram: anchor.Program<MerkleTreeProgram> =
   new anchor.Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

 var depositAmount, depositFeeAmount;
 const verifiers = [new VerifierZero(), new VerifierOne()];

 before(async () => {
   await createTestAccounts(provider.connection, userTokenAccount);
   LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
   await setUpMerkleTree(provider);

   POSEIDON = await circomlibjs.buildPoseidonOpt();

   KEYPAIR = new Account({
     poseidon: POSEIDON,
     seed: KEYPAIR_PRIVKEY.toString(),
   });

   const relayerRecipient = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(relayerRecipient, 2_000_000_000);

    RELAYER = await new TestRelayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      LOOK_UP_TABLE,
      relayerRecipient,
      new BN(100000),
    );

   depositAmount =
     10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
   depositFeeAmount =
     10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);

   for (var verifier in verifiers) {
     console.log("verifier ", verifier.toString());

     await token.approve(
       provider.connection,
       ADMIN_AUTH_KEYPAIR,
       userTokenAccount,
       Transaction.getSignerAuthorityPda(
         merkleTreeProgramId,
         verifiers[verifier].verifierProgram!.programId
       ), //delegate
       USER_TOKEN_ACCOUNT, // owner
       depositAmount * 10,
       [USER_TOKEN_ACCOUNT]
     );

     let lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    }); // userKeypair

     deposit_utxo1 = new Utxo({
       poseidon: POSEIDON,
       assets: [FEE_ASSET, MINT
     ],
       amounts: [
         new anchor.BN(depositFeeAmount),
         new anchor.BN(depositAmount),
       ],
       account: KEYPAIR,
     });

     let txParams = new TransactionParameters({
       outputUtxos: [deposit_utxo1],
       merkleTreePubkey: MERKLE_TREE_KEY,
       sender: userTokenAccount,
       senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
       verifier: verifiers[verifier],
       poseidon: POSEIDON,
       action: Action.SHIELD,
       lookUpTable: LOOK_UP_TABLE
     });

     var transaction = new Transaction({
       provider: lightProvider,
       params: txParams
     });

     await transaction.compileAndProve();
     await transaction.provider.provider.connection.confirmTransaction(
       await transaction.provider.provider.connection.requestAirdrop(
         transaction.params.accounts.authority,
         1_000_000_000
       ),
       "confirmed"
     );
     // does one successful transaction
     await transaction.sendAndConfirmTransaction();
     await lightProvider.relayer.updateMerkleTree(lightProvider);

     // // Deposit
     var deposit_utxo2 = new Utxo({
       poseidon: POSEIDON,
       assets: [FEE_ASSET, MINT
       ],
       amounts: [
         new anchor.BN(depositFeeAmount),
         new anchor.BN(depositAmount),
       ],
       account: KEYPAIR,
     });

     let txParams1 = new TransactionParameters({
       outputUtxos: [deposit_utxo2],
       merkleTreePubkey: MERKLE_TREE_KEY,
       sender: userTokenAccount,
       senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
       verifier: verifiers[verifier],
       poseidon: POSEIDON,
       action: Action.SHIELD,
       lookUpTable: LOOK_UP_TABLE
     });

     var transaction1 = new Transaction({
       provider: lightProvider,
       params: txParams1,
     });
     await transaction1.compileAndProve();
     transactions.push(transaction1);

     // Withdrawal
     var tokenRecipient = recipientTokenAccount;

     let lightProviderWithdrawal = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
    }); // userKeypair

     const relayerRecipient = SolanaKeypair.generate().publicKey;
     await provider.connection.confirmTransaction(
       await provider.connection.requestAirdrop(relayerRecipient, 10000000)
     );
     let relayer = new Relayer(
       ADMIN_AUTH_KEYPAIR.publicKey,
       lightProvider.lookUpTable,
       relayerRecipient,
       new BN(100000)
     );


     let txParams2 = new TransactionParameters({
       inputUtxos: [deposit_utxo1],
       merkleTreePubkey: MERKLE_TREE_KEY,
       recipient: tokenRecipient,
       recipientFee: ADMIN_AUTH_KEYPAIR.publicKey,
       verifier: verifiers[verifier],
       relayer,
       poseidon: POSEIDON,
       action: Action.UNSHIELD
     });
     var tx = new Transaction({
       provider: lightProviderWithdrawal,
       params: txParams2,
     });

     await tx.compileAndProve();
     // await tx.getRootIndex();
     // await tx.getPdaAddresses();
     transactions.push(tx);
     console.log(transactions[0].remainingAccounts)
   }
 });

 // afterEach(async () => {
 //   // Check that no nullifier was inserted, otherwise the prior test failed
 //   for (var tx in transactions) {
 //     await checkNfInserted(
 //       transactions[tx].params.nullifierPdaPubkeys,
 //       provider.connection
 //     );
 //   }
 // });

 const sendTestTx = async (
   tx: Transaction,
   type: string,
   account?: string
 ) => {
   var instructions = await tx.params.verifier.getInstructions(tx);
   console.log("aftere instructions");
   const provider = anchor.AnchorProvider.local(
     "http://127.0.0.1:8899",
     confirmConfig
   );
   tx.provider.provider = provider;
   // if (tx.app_params){
   //     console.log("tx.app_params ", tx.app_params);

   //     instructions = await tx.app_params.verifier.getInstructions(tx);
   // } else {
   //     instructions = await tx.params.verifier.getInstructions(tx);
   // }
   var e;

   for (var ix = 0; ix < instructions.length; ix++) {
     console.log("ix ", ix);
     if (ix != instructions.length - 1) {
       e = await tx.sendTransaction(instructions[ix]);

       // // confirm throws socket hangup error thus waiting a second instead
       await new Promise((resolve) => setTimeout(resolve, 700));
     } else {
       e = await tx.sendTransaction(instructions[ix]);
     }
   }
   console.log(e);

   if (type === "ProofVerificationFails") {
     assert.isTrue(
       e.logs.includes("Program log: error ProofVerificationFailed")
     );
   } else if (type === "Account") {
     assert.isTrue(
       e.logs.includes(
         `Program log: AnchorError caused by account: ${account}. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.`
       )
     );
   } else if (type === "preInsertedLeavesIndex") {
     assert.isTrue(
       e.logs.includes(
         "Program log: AnchorError caused by account: pre_inserted_leaves_index. Error Code: AccountDiscriminatorMismatch. Error Number: 3002. Error Message: 8 byte discriminator did not match what was expected."
       )
     );
   } else if (type === "Includes") {
     console.log("trying includes: ", account);

     assert.isTrue(e.logs.includes(account));
   }
   if (instructions.length > 1) {
     await tx.closeVerifierState();
   }
 };

 it("Wrong amount", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     let wrongAmount = new anchor.BN("123213").toArray();
     tmp_tx.transactionInputs.publicInputs.publicAmount = Array.from([
       ...new Array(29).fill(0),
       ...wrongAmount,
     ]);
     console.log("before sendTestTxs");

     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong feeAmount", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     let wrongFeeAmount = new anchor.BN("123213").toArray();
     tmp_tx.transactionInputs.publicInputs.feeAmount = Array.from([
       ...new Array(29).fill(0),
       ...wrongFeeAmount,
     ]);
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong Mint", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     let relayer = SolanaKeypair.generate();
     const newMintKeypair = SolanaKeypair.generate();
     await createMintWrapper({
       authorityKeypair: ADMIN_AUTH_KEYPAIR,
       mintKeypair: newMintKeypair,
       connection: provider.connection,
     });
     tmp_tx.params.accounts.sender = await newAccountWithTokens({
       connection: provider.connection,
       MINT: newMintKeypair.publicKey,
       ADMIN_AUTH_KEYPAIR,
       userAccount: relayer,
       amount: new BN(0),
     });
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong encryptedUtxos", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     tmp_tx.params.encryptedUtxos = new Uint8Array(174).fill(2);
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong relayerFee", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     tmp_tx.params.relayer.relayerFee = new anchor.BN("9000");
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong nullifier", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     for (var i in tmp_tx.transactionInputs.publicInputs.nullifiers) {
       tmp_tx.transactionInputs.publicInputs.nullifiers[i] = new Array(32).fill(2);
       await sendTestTx(tmp_tx, "ProofVerificationFails");
     }
   }
 });

 it("Wrong leaves", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     for (var i in tmp_tx.transactionInputs.publicInputs.leaves) {
       tmp_tx.transactionInputs.publicInputs.leaves[0][i] = new Array(32).fill(2);
       await sendTestTx(tmp_tx, "ProofVerificationFails");
     }
   }
 });

 // doesn't work sig verify error
 it.skip("Wrong signer", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     const wrongSinger = SolanaKeypair.generate();
     await provider.connection.confirmTransaction(
       await provider.connection.requestAirdrop(
         wrongSinger.publicKey,
         1_000_000_000
       ),
       "confirmed"
     );
     tmp_tx.provider.wallet = useWallet(wrongSinger);
     tmp_tx.params.relayer.accounts.relayerPubkey = wrongSinger.publicKey;
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong recipientFee", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     tmp_tx.params.accounts.recipientFee = SolanaKeypair.generate().publicKey;
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong recipient", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     tmp_tx.params.accounts.recipient = SolanaKeypair.generate().publicKey;
     await sendTestTx(tmp_tx, "ProofVerificationFails");
   }
 });

 it("Wrong registeredVerifierPda", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     if (
       tmp_tx.params.accounts.registeredVerifierPda.toBase58() ==
       REGISTERED_VERIFIER_ONE_PDA.toBase58()
     ) {
       tmp_tx.params.accounts.registeredVerifierPda = REGISTERED_VERIFIER_PDA;
     } else {
       tmp_tx.params.accounts.registeredVerifierPda =
         REGISTERED_VERIFIER_ONE_PDA;
     }
     await sendTestTx(tmp_tx, "Account", "registered_verifier_pda");
   }
 });

 it("Wrong authority", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     tmp_tx.params.accounts.authority = Transaction.getSignerAuthorityPda(
       merkleTreeProgramId,
       SolanaKeypair.generate().publicKey
     );
     await sendTestTx(tmp_tx, "Account", "authority");
   }
 });

 it("Wrong nullifier accounts", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     // await tmp_tx.getPdaAddresses();
     for (var i = 0; i < tmp_tx.remainingAccounts.nullifierPdaPubkeys.length; i++) {
       tmp_tx.remainingAccounts.nullifierPdaPubkeys[i] =
         tmp_tx.remainingAccounts.nullifierPdaPubkeys[
           (i + 1) % tmp_tx.remainingAccounts.nullifierPdaPubkeys.length
         ];
       await sendTestTx(
         tmp_tx,
         "Includes",
         "Program log: Passed-in pda pubkey != on-chain derived pda pubkey."
       );
     }
   }
 });

 it("Wrong leavesPdaPubkeys accounts", async () => {
   for (var tx in transactions) {
     var tmp_tx: Transaction = _.cloneDeep(transactions[tx]);
     await tmp_tx.getPdaAddresses();
     if (tmp_tx.remainingAccounts.leavesPdaPubkeys.length > 1) {
       for (var i = 0; i < tmp_tx.remainingAccounts.leavesPdaPubkeys.length; i++) {
         tmp_tx.remainingAccounts.leavesPdaPubkeys[i] =
           tmp_tx.remainingAccounts.leavesPdaPubkeys[
             (i + 1) % tmp_tx.remainingAccounts.leavesPdaPubkeys.length
           ];
         await sendTestTx(
           tmp_tx,
           "Includes",
           "Program log: Instruction: InsertTwoLeaves"
         );
       }
     } else {
       tmp_tx.remainingAccounts.leavesPdaPubkeys[0] = {
         isSigner: false,
         isWritable: true,
         pubkey: SolanaKeypair.generate().publicKey,
       };
       await sendTestTx(
         tmp_tx,
         "Includes",
         "Program log: AnchorError caused by account: two_leaves_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
       );
     }
   }
 });
});
