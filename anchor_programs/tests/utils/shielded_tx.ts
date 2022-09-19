const light = require('../../light-protocol-sdk');
const {U64, I64} = require('n64');
const anchor = require("@project-serum/anchor")
const nacl = require('tweetnacl')
const FIELD_SIZE = new anchor.BN('21888242871839275222246405745257275088548364400416034343698204186575808495617');
export const createEncryptionKeypair = () => nacl.box.keyPair()
var assert = require('assert');
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
  } from "./constants";
import { PublicKey, Keypair, SystemProgram, TransactionMessage, ComputeBudgetProgram,  AddressLookupTableAccount, VersionedTransaction, sendAndConfirmRawTransaction } from "@solana/web3.js";
import { newAccountWithLamports  } from "./test_transactions";
import { TOKEN_PROGRAM_ID, getAccount  } from '@solana/spl-token';
import {checkRentExemption} from './test_checks';
import {unpackLeavesAccount} from './unpack_accounts';
export class shieldedTransaction {
  constructor({
    keypair, // : Keypair shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    encryptionKeypair = createEncryptionKeypair(),
    relayerFee = U64(10_000),
    merkleTreeIndex = 0,
    merkleTreePubkey,
    merkleTree = null,
    merkleTreeAssetPubkey = null,
    poseidon = null,
    recipient, //PublicKey
    // recipientFee: number,
    lookupTable, //PublicKey
    payer, //: Keypair
    relayerPubkey = null, //PublicKey
    merkleTreeProgram, // any
    verifierProgram,//: any
    merkle_tree_token_pda,
    preInsertedLeavesIndex,
    provider,
    merkleTreeFeeAssetPubkey
  }) {
      if (keypair == null) {
        keypair = new light.Keypair(poseidon);
      } else {
        this.keypair = keypair;
      }
      if (relayerPubkey == null) {
          this.relayerPubkey = new PublicKey(payer.publicKey);
      } else {
         this.relayerPubkey = relayerPubkey;
      }
      console.log("payer: ", payer);

      console.log("this.relayerPubkey ", this.relayerPubkey);

      this.preInsertedLeavesIndex = preInsertedLeavesIndex;
      this.merkleTreeProgram = merkleTreeProgram;
      this.verifierProgram = verifierProgram;
      this.lookupTable = lookupTable;
      this.feeAsset = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE);
      this.relayerFee = relayerFee;
      this.merkleTreeIndex = merkleTreeIndex;
      this.merkleTreePubkey = merkleTreePubkey;
      this.merkleTreeAssetPubkey = merkleTreeAssetPubkey;
      this.merkleTree = null;
      this.utxos = [];
      this.feeUtxos = [];
      this.encryptionKeypair = encryptionKeypair;
      this.poseidon = poseidon;
      this.payer = payer;
      this.provider = provider;
      this.recipient = recipient;
      this.merkleTreeFeeAssetPubkey = merkleTreeFeeAssetPubkey;

    }

    async getMerkleTree() {
      this.merkleTree = await light.buildMerkelTree(this.poseidon);
      this.merkleTreeLeavesIndex = 0;
    }

    prepareUtxos({
      inputUtxos,
      outputUtxos,
      action,
      assetPubkeys,
      recipient,
      mintPubkey = 0,
      relayerFee, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
      shuffle = true,
      recipientFee = null,
      sender
    }) {
      // mintPubkey = assetPubkeys[1];
      // if (assetPubkeys[1].toString() != mintPubkey.toString()) {
      //   throw "mintPubkey should be assetPubkeys[1]";
      // }
      if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
        throw "feeAsset should be assetPubkeys[0]";
      }

      if (action == "DEPOSIT") {
        this.relayerFee = relayerFee;
        // this.sender = this.payer.publicKey;
        this.recipientFee = this.merkleTreeFeeAssetPubkey;
        this.sender = sender;
        this.recipient = this.merkleTreeAssetPubkey;
        if (this.relayerPubkey != new PublicKey(this.payer.publicKey)) {
          throw "relayerPubkey and payer pubkey need to be equivalent at deposit";
        }
      } else if (action == "WITHDRAWAL") {
        this.relayerFee = relayerFee;
        this.sender = this.merkleTreeAssetPubkey;
        this.recipient = recipient;
        this.recipientFee = recipientFee;
      }


      this.assetPubkeys = assetPubkeys;
      this.mintPubkey = mintPubkey;
      this.action = action;
      let res = light.prepareUtxos(
          inputUtxos,
          outputUtxos,
          this.relayerFee,
          this.assetPubkeys,
          this.action,
          this.poseidon,
          shuffle
      );

      this.inputUtxos = res.inputUtxos;
      this.outputUtxos = res.outputUtxos;
      this.inIndices = res.inIndices;
      this.outIndices = res.outIndices;
      this.externalAmountBigNumber = res.externalAmountBigNumber;
      if (this.externalAmountBigNumber != 0) {
        console.log(assetPubkeys[1]);

        if (assetPubkeys[1].toString() != mintPubkey.toString()) {
          throw "mintPubkey should be assetPubkeys[1]";
        }
      }
      // console.log("this.inputUtxos[0]: ", this.inputUtxos[0])
      // console.log("this.inputUtxos[1]: ", this.inputUtxos[1])
      // console.log("this.inputUtxos[2]: ", this.inputUtxos[2])
      // console.log("this.inputUtxos[3]: ", this.inputUtxos[3])
      //
      // console.log("this.outputUtxos[0]: ", this.outputUtxos[0])
      // console.log("this.outputUtxos[1]: ", this.outputUtxos[1])
      // console.log("this.outputUtxos[2]: ", this.outputUtxos[2])
      // console.log("this.outputUtxos[3]: ", this.outputUtxos[3])
      //
      // console.log("this.inIndices: ", this.inIndices)
      // console.log("this.outIndices: ", this.outIndices)
      // console.log("this.externalAmountBigNumber: ", this.externalAmountBigNumber)


    }

    async prepareTransaction() {
      let data = await light.prepareTransaction(
       this.inputUtxos,
       this.outputUtxos,
       this.merkleTree,
       this.merkleTreeIndex,
       this.merkleTreePubkey.toBytes(),
       this.externalAmountBigNumber,
       this.relayerFee,
       this.recipient, // recipient
       this.relayerPubkey,
       this.action,
       this.encryptionKeypair,
       this.inIndices,
       this.outIndices,
       this.assetPubkeys,
       this.mintPubkey,
       false,
       this.feeAmount,
       this.recipientFee
     )
     this.input = data.input;
     this.extAmount = data.extAmount;
     this.externalAmountBigNumber = data.externalAmountBigNumber;
     this.extDataBytes = data.extDataBytes;
     this.encryptedOutputs = data.extDataBytes;
    }

    async prepareTransactionFull({
      inputUtxos,
      outputUtxos,
      action,
      assetPubkeys,
      recipient,
      mintPubkey = 0,
      relayerFee, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
      shuffle = true,
      recipientFee = null,
      sender
    }) {
      mintPubkey = assetPubkeys[1];
      if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
        throw "feeAsset should be assetPubkeys[0]";
      }
      if (action == "DEPOSIT") {
        this.relayerFee = relayerFee;
        console.log(this.payer);

        this.sender = sender;
        console.log("setting recipient to this.merkleTreeAssetPubkey");
        this.senderFee  = new PublicKey(this.payer.publicKey);
        this.recipient = this.merkleTreeAssetPubkey;
        console.log("this.recipient: ", this.recipient);
        console.log("this.merkleTreeAssetPubkey: ", this.merkleTreeAssetPubkey);
        this.recipientFee = this.merkleTreeFeeAssetPubkey;

        if (this.relayerPubkey.toBase58() != new PublicKey(this.payer.publicKey).toBase58()) {
          throw "relayerPubkey and payer pubkey need to be equivalent at deposit";
        }
      } else if (action == "WITHDRAWAL") {
        this.relayerFee = relayerFee;
        this.sender = this.merkleTreeAssetPubkey;
        this.recipient = recipient;
        this.senderFee = this.merkleTreeFeeAssetPubkey;
      }

      this.assetPubkeys = assetPubkeys;
      this.mintPubkey = mintPubkey;
      this.action = action;

      let res = light.prepareUtxos(
          inputUtxos,
          outputUtxos,
          this.relayerFee,
          this.assetPubkeys,
          this.action,
          this.poseidon,
          shuffle
      );
      this.inputUtxos = res.inputUtxos;
      this.outputUtxos = res.outputUtxos;
      this.inIndices = res.inIndices;
      this.outIndices = res.outIndices;
      this.externalAmountBigNumber = res.externalAmountBigNumber;
      this.feeAmount = res.feeAmount;

      let data = await light.prepareTransaction(
       this.inputUtxos,
       this.outputUtxos,
       this.merkleTree,
       this.merkleTreeIndex,
       this.merkleTreePubkey.toBytes(),
       this.externalAmountBigNumber,
       this.relayerFee,
       this.merkleTreeAssetPubkey,
       this.relayerPubkey,
       this.action,
       this.encryptionKeypair,
       this.inIndices,
       this.outIndices,
       this.assetPubkeys,
       this.mintPubkey,
       false,
       this.feeAmount,
       this.recipientFee
     )
     this.input = data.input;
     assert(this.input.mintPubkey == this.mintPubkey);
     assert(this.input.mintPubkey == this.assetPubkeys[1]);
     this.extAmount = data.extAmount;
     this.externalAmountBigNumber = data.externalAmountBigNumber;
     this.extDataBytes = data.extDataBytes;
     this.encrypedUtxos = data.encryptedUtxos
     if (this.externalAmountBigNumber != 0) {
       if (assetPubkeys[1].toString() != mintPubkey.toString()) {
         throw "mintPubkey should be assetPubkeys[1]";
       }
     }
    }

    async proof() {
      if (this.merkleTree == null) {
        throw "merkle tree not built";
      }
      if (this.inIndices == null) {
        throw "transaction not prepared";
      }
      let proofData = await light.getProofMasp(
        this.input,
        this.extAmount,
        this.externalAmountBigNumber,
        this.extDataBytes,
        this.encrypedUtxos
      )

      this.proofData = proofData;

      let pdas = await getPdaAddresses({
        tx_integrity_hash: this.proofData.publicInputs.txIntegrityHash,
        nullifiers: [this.proofData.publicInputs.nullifier0, this.proofData.publicInputs.nullifier1],
        leftLeaves: [this.proofData.publicInputs.leafLeft],
        merkleTreeProgram: this.merkleTreeProgram,
        verifierProgram: this.verifierProgram
      })
      this.escrow = pdas.escrow;
      this.leavesPdaPubkeys = pdas.leavesPdaPubkeys;
      this.nullifierPdaPubkeys = pdas.nullifierPdaPubkeys;
      this.signerAuthorityPubkey = pdas.signerAuthorityPubkey;
      return this.proofData;
    }

    async sendTransaction(insert = true){
      console.log("this.relayerFee ", this.relayerFee);

      this.recipientBalancePriorTx = (await getAccount(
        this.provider.connection,
        this.merkleTreeAssetPubkey,
        TOKEN_PROGRAM_ID
      )).amount;
      this.recipientFeeBalancePriorTx = await this.provider.connection.getBalance(this.recipientFee);
      console.log("recipientBalancePriorTx: ", this.recipientBalancePriorTx);
      console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);
      console.log("sender_fee: ", this.senderFee);
      this.senderFeeBalancePriorTx = await this.provider.connection.getBalance(this.senderFee);

      console.log("this.recipient: ", this.recipient);

      const ix = await this.verifierProgram.methods.shieldedTransferInputs(
        this.proofData.proofBytes,
        this.proofData.publicInputs.root,
        this.proofData.publicInputs.publicAmount.slice(24,32),
        this.proofData.publicInputs.extDataHash,
        [this.proofData.publicInputs.nullifier0,this.proofData.publicInputs.nullifier1],
        [this.proofData.publicInputs.leafRight, this.proofData.publicInputs.leafLeft],
        this.proofData.publicInputs.feeAmount.slice(24,32),
        this.proofData.publicInputs.mintPubkey,
        new anchor.BN(1), //this.proofData.merkleTreeIndex,
        new anchor.BN(0),
        new anchor.BN(this.relayerFee.toString()),// relayer_fee
        this.proofData.encryptedOutputs.slice(0,128),
        this.proofData.encryptedOutputs.slice(128,192),
        this.proofData.encryptedOutputs.slice(192,224),
        this.proofData.encryptedOutputs.slice(224,238)
      ).accounts(
        {
          signingAddress:     this.relayerPubkey,
          systemProgram:      SystemProgram.programId,
          programMerkleTree:  this.merkleTreeProgram.programId,
          rent:               DEFAULT_PROGRAMS.rent,
          merkleTree:         this.merkleTreePubkey,//MERKLE_TREE_USDC,
          preInsertedLeavesIndex: this.preInsertedLeavesIndex,//PRE_INSERTED_LEAVES_INDEX_USDC,
          authority:          this.signerAuthorityPubkey,
          tokenProgram:       TOKEN_PROGRAM_ID,
          sender:             this.sender,
          recipient:          this.recipient, //MERKLE_TREE_PDA_TOKEN_USDC,
          senderFee:          this.senderFee,
          recipientFee:       this.recipientFee,
          relayerRecipient:   this.signerAuthorityPubkey, //AUTHORITY, // doesnt matter at deposit is not called
          escrow:             this.escrow,
        }
      )
      .remainingAccounts([
        { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[0]},
        { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[1]},
        { isSigner: false, isWritable: true, pubkey: this.leavesPdaPubkeys[0]}
      ])
      .signers([this.payer]).instruction()

      let recentBlockhash = (await this.provider.connection.getRecentBlockhash()).blockhash;
      let txMsg = new TransactionMessage({
            payerKey: this.payer.publicKey,
            instructions: [
              // ComputeBudgetProgram.requestUnits({units:1_400_000, fee: 1}),
              // SystemProgram.transfer({fromPubkey:payer.publicKey, toPubkey: AUTHORITY, lamports: 3173760 * 3}),
              ix
            ],
            recentBlockhash: recentBlockhash})
      console.log(this.lookupTable.toBase58());

      let lookupTableAccount = await this.provider.connection.getAccountInfo(this.lookupTable, "confirmed");
      console.log("lookupTableAccount: ", lookupTableAccount);

      let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(lookupTableAccount.data);
      console.log("unpackedLookupTableAccount ", unpackedLookupTableAccount);

      let compiledTx = txMsg.compileToV0Message([{state: unpackedLookupTableAccount}]);
      compiledTx.addressTableLookups[0].accountKey = this.lookupTable

      let transaction = new VersionedTransaction(compiledTx);
      transaction.sign([this.payer])
      console.log(transaction);
      console.log(transaction.message.addressTableLookups);
      recentBlockhash = (await this.provider.connection.getRecentBlockhash()).blockhash;
      transaction.message.recentBlockhash = recentBlockhash;
      let serializedTx = transaction.serialize();
      // console.log(this.provider.connection);
      let res
      try {
        console.log("serializedTx: ", Array.from(serializedTx).toString());

        res = await sendAndConfirmRawTransaction(this.provider.connection, serializedTx,
          {
            commitment: 'finalized',
            preflightCommitment: 'finalized',
          }
        );

      } catch (e) {
        console.log(e);

      }

      // storing utxos
      this.outputUtxos.map((utxo) => {
        if (utxo.amounts[1] != 0 && utxo.assets[1] != this.feeAsset) {
            this.utxos.push(utxo)
        }
        if (utxo.amounts[0] != 0 && utxo.assets[0].toString() == this.feeAsset.toString()) {
          this.feeUtxos.push(utxo)
        }
      })
      this.inIndices = null;
      // inserting output utxos into merkle tree
      if (insert != "NOINSERT") {
        for (var i = 0; i<this.outputUtxos.length; i++) {
          this.merkleTree.update(this.merkleTreeLeavesIndex, this.outputUtxos[i].getCommitment())
          this.merkleTreeLeavesIndex++;
        }
      }

      return res;
    }

    async checkBalances(){
      // Checking that nullifiers were inserted
      console.log("here");
      this.is_token = true;

      for (var i in this.nullifierPdaPubkeys) {
        console.log(i);

        var nullifierAccount = await this.provider.connection.getAccountInfo(
          this.nullifierPdaPubkeys[i],
          {
          commitment: 'confirmed',
          preflightCommitment: 'confirmed',
        }
        );
        console.log(nullifierAccount);

        await checkRentExemption({
          account: nullifierAccount,
          connection: this.provider.connection
        });
      }
      console.log("here1");
      let leavesAccount
      var leavesAccountData
      console.log("this.leavesPdaPubkeys: ", this.leavesPdaPubkeys);

      // Checking that leaves were inserted
      for (var i in this.leavesPdaPubkeys) {
        console.log(i);

        leavesAccount = await this.provider.connection.getAccountInfo(
          this.leavesPdaPubkeys[i]
        );
        console.log(leavesAccount);

        leavesAccountData = unpackLeavesAccount(leavesAccount.data)
        await checkRentExemption({
          account: leavesAccount,
          connection: this.provider.connection
        });
        try {
          console.log(leavesAccountData);

          assert(leavesAccountData.leafLeft.toString() === this.proofData.publicInputs.leafLeft.toString(), "left leaf not inserted correctly")
          assert(leavesAccountData.leafRight.toString() === this.proofData.publicInputs.leafRight.toString(), "right leaf not inserted correctly")
          assert(leavesAccountData.encryptedUtxos.toString() === this.encrypedUtxos.toString(), "encryptedUtxos not inserted correctly")
          assert(leavesAccountData.leafType  === 7);
        } catch(e) {
          console.log("leaves: ", e);
        }
      }

      console.log("here2");
      console.log(`mode ${this.action}, this.is_token ${this.is_token}`);

      try {
        console.log("this.preInsertedLeavesIndex ", this.preInsertedLeavesIndex);

        var preInsertedLeavesIndexAccount = await this.provider.connection.getAccountInfo(
          this.preInsertedLeavesIndex
        )

        console.log(preInsertedLeavesIndexAccount);
        const preInsertedLeavesIndexAccountAfterUpdate = this.merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('PreInsertedLeavesIndex', preInsertedLeavesIndexAccount.data);

        assert(Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) == Number(leavesAccountData.leafIndex) + this.leavesPdaPubkeys.length * 2)

      } catch(e) {
        console.log("preInsertedLeavesIndex: ", e);

      }

      if (this.action == "DEPOSIT" && this.is_token == false) {
        var recipientAccount = await this.provider.connection.getAccountInfo(this.recipient)
        assert(recipientAccount.lamports == (I64(this.recipientBalancePriorTx).add(this.proofData.externalAmountBigNumber.toString())).toString(), "amount not transferred correctly");

      } else if (this.action == "DEPOSIT" && this.is_token == true) {
        console.log("DEPOSIT and token");
        console.log("this.recipient: ", this.recipient);

          var recipientAccount = await getAccount(
          this.provider.connection,
          this.recipient,
          TOKEN_PROGRAM_ID
        );
        var recipientFeeAccountBalance = await this.provider.connection.getBalance(this.recipientFee);

        // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
        // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.proofData.extAmount, 0)).toString(), "amount not transferred correctly");

        console.log(`Balance now ${recipientAccount.amount} balance beginning ${this.recipientBalancePriorTx}`)
        console.log(`Balance now ${recipientAccount.amount} balance beginning ${(Number(this.recipientBalancePriorTx) + Number(this.proofData.externalAmountBigNumber))}`)
        assert(recipientAccount.amount == (Number(this.recipientBalancePriorTx) + Number(this.proofData.externalAmountBigNumber)).toString(), "amount not transferred correctly");
        console.log(`Blanace now ${recipientFeeAccountBalance} ${Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)}`);
        console.log("fee amount: ", this.feeAmount);
        console.log("fee amount from inputs. ", new anchor.BN(this.proofData.publicInputs.feeAmount.slice(24,32)).toString());
        console.log("pub amount from inputs. ", new anchor.BN(this.proofData.publicInputs.publicAmount.slice(24,32)).toString());

        console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);

        var senderFeeAccountBalance = await this.provider.connection.getBalance(this.senderFee);
        console.log("senderFeeAccountBalance: ", senderFeeAccountBalance);
        console.log("this.senderFeeBalancePriorTx: ", this.senderFeeBalancePriorTx);

        assert(recipientFeeAccountBalance == Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount));
        console.log(`${Number(this.senderFeeBalancePriorTx)} - ${Number(this.feeAmount)} == ${senderFeeAccountBalance}`);
        assert(Number(this.senderFeeBalancePriorTx) - Number(this.feeAmount) - 5000 == Number(senderFeeAccountBalance) );

      } else if (this.action == "withdrawal" && this.is_token == false) {
        var senderAccount = await this.provider.connection.getAccountInfo(this.sender)
        var recipientAccount = await this.provider.connection.getAccountInfo(this.recipient)
        // console.log("senderAccount.lamports: ", senderAccount.lamports)
        // console.log("I64(senderAccountBalancePriorLastTx): ", I64(senderAccountBalancePriorLastTx).toString())
        // console.log("Sum: ", ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString())

        assert(senderAccount.lamports == ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");

        var recipientAccount = await this.provider.connection.getAccountInfo(recipient)
        // console.log(`recipientAccount.lamports: ${recipientAccount.lamports} == sum ${((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.proofData.extAmount, 0))).add(I64(relayerFee))).toString()}
        // Number(this.recipientBalancePriorTx): ${Number(this.recipientBalancePriorTx)}
        // relayerFee: ${Number(relayerFee)}
        // `)
        assert(recipientAccount.lamports == ((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.proofData.extAmount, 0)))).toString(), "amount not transferred correctly");
        // var relayerAccount = await this.provider.connection.getAccountInfo(
        //   relayer
        // )
        // console.log("relayer: ", relayer.toBase58())
        // let rent_verifier = await connection.getMinimumBalanceForRentExemption(5120)
        // // let rent_escrow = await connection.getMinimumBalanceForRentExemption(256)
        // let rent_nullifier = await connection.getMinimumBalanceForRentExemption(0)
        // let rent_leaves = await connection.getMinimumBalanceForRentExemption(256)
        // console.log("rent_verifier: ", rent_verifier)
        // console.log("rent_nullifier: ", rent_nullifier)
        // console.log("rent_leaves: ", rent_leaves)
        //
        // let expectedBalanceRelayer = I64(relayerFee)
        //   .add(I64(Number(relayerAccountBalancePriorLastTx)))
        //   .add(I64(Number(rent_verifier)))
        //   // .add(I64(Number(rent_escrow)))
        //   .sub(I64(Number(rent_nullifier)))
        //   .sub(I64(Number(rent_nullifier)))
        //   .sub(I64(Number(rent_leaves)))
        // console.log("relayerAccountBalancePriorLastTx: ", relayerAccountBalancePriorLastTx)
        // console.log(`${relayerAccount.lamports } == ${expectedBalanceRelayer}`)
        // assert(relayerAccount.lamports == expectedBalanceRelayer.toString())

      }  else if (this.action == "withdrawal" && this.is_token == true) {
        var senderAccount = await getAccount(
          this.provider.connection,
          this.sender,
          TOKEN_PROGRAM_ID
        );
        var recipientAccount = await getAccount(
          this.provider.connection,
          this.recipient,
          TOKEN_PROGRAM_ID
        );

        var relayerAccount = await getAccount(
          this.provider.connection,
          this.relayer,
          TOKEN_PROGRAM_ID
        );
        assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
        console.log(`${recipientAccount.amount}, ${Number(this.recipientBalancePriorTx)} ${I64.readLE(this.proofData.extAmount, 0)} ${I64(relayerFee)}`)
        assert(recipientAccount.amount == ((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.proofData.extAmount, 0)))).toString(), "amount not transferred correctly");
        console.log(`relayerAccount.amount ${relayerAccount.amount} == I64(relayerFee) ${I64(relayerFee)} + ${relayerAccountBalancePriorLastTx}`)
        assert(relayerAccount.amount == (I64(relayerFee).add(I64(Number(relayerAccountBalancePriorLastTx)))).toString())
      } else {
        throw Error("mode not supplied");
      }
    }


}


export async function getPdaAddresses({
  tx_integrity_hash,
  nullifiers,
  leftLeaves,
  merkleTreeProgram,
  verifierProgram
}) {
  console.log("new Uint8Array(nullifier0) ", new Uint8Array(nullifiers[0]));

  let nullifierPdaPubkeys = [];
  for (var i in nullifiers) {
    nullifierPdaPubkeys.push(
    (await PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(nullifiers[i])), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId))[0]);
  }

  let leavesPdaPubkeys = [];
  for (var i in leftLeaves) {
    leavesPdaPubkeys.push(
    (await PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(leftLeaves[i])), anchor.utils.bytes.utf8.encode("leaves")],
        merkleTreeProgram.programId))[0]);
  }

  return {
    signerAuthorityPubkey: (await PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBytes()],
        verifierProgram.programId))[0],

    escrow: (await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("escrow")],
        verifierProgram.programId))[0],
    verifierStatePubkey: (await PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("storage")],
        verifierProgram.programId))[0],
    feeEscrowStatePubkey: (await PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("escrow")],
        verifierProgram.programId))[0],
    merkleTreeUpdateState: (await PublicKey.findProgramAddress(
        [Buffer.from(new Uint8Array(leftLeaves[0])), anchor.utils.bytes.utf8.encode("storage")],
        merkleTreeProgram.programId))[0],
    nullifierPdaPubkeys,
    leavesPdaPubkeys
  }
}
