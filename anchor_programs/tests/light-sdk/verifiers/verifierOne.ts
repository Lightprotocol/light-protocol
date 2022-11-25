import { VerifierProgramOne } from "../idls/verifier_program_one";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";


export class VerifierOne {
  constructor() {
    this.verifierProgram = anchor.workspace.VerifierProgramOne as Program<VerifierProgramOne>;
    this.wtnsGenPath = "./Light_circuits/build/circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = './Light_circuits/build/circuits/transactionMasp10'
    this.calculateWtns = require('../../../Light_circuits/build/circuits/transactionMasp10_js/witness_calculator.js')

  }

  async transferFirst(this) {
    console.log("in transferFirst");
    if (publicInputsBytes.length == 17) {
        publicInputs = {
         root:         publicInputsBytes[0],
         publicAmount: publicInputsBytes[1],
         extDataHash:  publicInputsBytes[2],
         feeAmount:    publicInputsBytes[3],
         mintPubkey:   publicInputsBytes[4],
         nullifiers:   Array.from(publicInputsBytes.slice(5,15)),
         leaves:     [publicInputsBytes[15], publicInputsBytes[16]]
       };
    } else {
      throw `publicInputsBytes.length invalid ${publicInputsBytes.length} != 17`;
    }

    const ix1 = await this.verifierProgram.methods.shieldedTransferFirst(
      Buffer.from(this.proofData.publicInputs.publicAmount),
      this.proofData.publicInputs.nullifiers,
      this.proofData.publicInputs.leaves,
      Buffer.from(this.proofData.publicInputs.feeAmount),
      new anchor.BN(this.root_index.toString()),
      new anchor.BN(this.relayerFee.toString()),
      Buffer.from(this.proofData.encryptedOutputs)
    ).accounts(
      {
        signingAddress:     this.relayerPubkey,
        systemProgram:      SystemProgram.programId,
        verifierState:      this.verifierStatePubkey
      }
    )
    .signers([this.payer])
    .rpc({
      commitment: 'finalized',
      preflightCommitment: 'finalized',
    });
    console.log("ix1 success ", ix1);
  }

  async transferSecond(this) {
    const ix = await this.verifierProgram.methods.shieldedTransferSecond(
      Buffer.from(this.proofData.proofBytes)
    ).accounts(
      {
        signingAddress:     this.relayerPubkey,
        verifierState:      this.verifierStatePubkey,
        systemProgram:      SystemProgram.programId,
        programMerkleTree:  this.merkleTreeProgram.programId,
        rent:               DEFAULT_PROGRAMS.rent,
        merkleTree:         this.merkleTreePubkey,
        preInsertedLeavesIndex: this.preInsertedLeavesIndex,
        authority:          this.signerAuthorityPubkey,
        tokenProgram:       TOKEN_PROGRAM_ID,
        sender:             this.sender,
        recipient:          this.recipient,
        senderFee:          this.senderFee,
        recipientFee:       this.recipientFee,
        relayerRecipient:   this.relayerRecipient,
        escrow:             this.escrow,
        tokenAuthority:     this.tokenAuthority,
        registeredVerifierPda: this.registeredVerifierPda
      }
    )
    .remainingAccounts([
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[0]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[1]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[2]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[3]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[4]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[5]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[6]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[7]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[8]},
      { isSigner: false, isWritable: true, pubkey: this.nullifierPdaPubkeys[9]},
      { isSigner: false, isWritable: true, pubkey: this.leavesPdaPubkeys[0]}
    ])
    .signers([this.payer]).instruction();
    let recentBlockhash = (await this.provider.connection.getRecentBlockhash("finalized")).blockhash;


    let txMsg = new TransactionMessage({
      payerKey: this.payer.publicKey,
      instructions: [
        ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
        ix
      ],
      recentBlockhash: recentBlockhash})

      let lookupTableAccount = await this.provider.connection.getAccountInfo(this.lookupTable, "confirmed");

      let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(lookupTableAccount.data);

      let compiledTx = txMsg.compileToV0Message([{state: unpackedLookupTableAccount}]);
      compiledTx.addressTableLookups[0].accountKey = this.lookupTable

      let transaction = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res
      while (retries > 0) {
        transaction.sign([this.payer])
        recentBlockhash = (await this.provider.connection.getRecentBlockhash("finalized")).blockhash;
        transaction.message.recentBlockhash = recentBlockhash;
        let serializedTx = transaction.serialize();

        try {
          console.log("serializedTx: ");

          res = await sendAndConfirmRawTransaction(this.provider.connection, serializedTx,
            {
              commitment: 'finalized',
              preflightCommitment: 'finalized',
            }
          );
          retries = 0;

        } catch (e) {
          console.log(e);
          retries--;
          if (retries == 0 || e.logs != undefined) {
            const ixClose = await this.verifierProgram.methods.closeVerifierState(
            ).accounts(
              {
                signingAddress:     this.relayerPubkey,
                verifierState:      this.verifierStatePubkey
              }
            )
            .signers([this.payer]).rpc({
              commitment: 'finalized',
              preflightCommitment: 'finalized',
            });
            return e;
          }
        }

      }
    }

  async sendTransaction(insert = true){
      assert(this.nullifierPdaPubkeys.length == 10);
      let balance = await this.provider.connection.getBalance(this.signerAuthorityPubkey, {preflightCommitment: "confirmed", commitment: "confirmed"});
      if (balance === 0) {
        await this.provider.connection.confirmTransaction(await this.provider.connection.requestAirdrop(this.signerAuthorityPubkey, 1_000_000_000), {preflightCommitment: "confirmed", commitment: "confirmed"})
      }
      try {
        this.recipientBalancePriorTx = (await getAccount(
          this.provider.connection,
          this.recipient,
          TOKEN_PROGRAM_ID
        )).amount;

      } catch (error) {

      }
      this.recipientFeeBalancePriorTx = await this.provider.connection.getBalance(this.recipientFee);
      // console.log("recipientBalancePriorTx: ", this.recipientBalancePriorTx);
      // console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);
      // console.log("sender_fee: ", this.senderFee);
      this.senderFeeBalancePriorTx = await this.provider.connection.getBalance(this.senderFee);
      this.relayerRecipientAccountBalancePriorLastTx = await this.provider.connection.getBalance(this.relayerRecipient);

      // console.log("signingAddress:     ", this.relayerPubkey)
      // console.log("systemProgram:      ", SystemProgram.programId)
      // console.log("programMerkleTree:  ", this.merkleTreeProgram.programId)
      // console.log("rent:               ", DEFAULT_PROGRAMS.rent)
      // console.log("merkleTree:         ", this.merkleTreePubkey)
      // console.log("preInsertedLeavesInd", this.preInsertedLeavesIndex)
      // console.log("authority:          ", this.signerAuthorityPubkey)
      // console.log("tokenProgram:       ", TOKEN_PROGRAM_ID)
      // console.log("sender:             ", this.sender)
      // console.log("recipient:          ", this.recipient)
      // console.log("senderFee:          ", this.senderFee)
      // console.log("recipientFee:       ", this.recipientFee)
      // console.log("relayerRecipient:   ", this.relayerRecipient)
      // console.log("escrow:             ", this.escrow)
      // console.log("tokenAuthority:     ", this.tokenAuthority)
      // console.log("registeredVerifierPd",this.registeredVerifierPda)
      // console.log("encryptedOutputs len ", this.proofData.encryptedOutputs.length);
      // console.log("this.proofData.encryptedOutputs[0], ", this.proofData.encryptedOutputs);
      console.log("this.verifierStatePubkey, ", this.verifierStatePubkey.toBase58());
      // console.log("this.proofData.publicInputs.nullifiers, ", this.proofData.publicInputs.nullifiers);
      // console.log("this.root_index ", this.root_index);
      // console.log("this.relayerFee ", this.relayerFee);
      // console.log("this.encryptedOutputs ", this.proofData.encryptedOutputs);
      this.transferFirst = transferFirst;
      this.transferSecond = transferSecond;

      let res = await this.transferFirst();
      res = await this.transferSecond();

      return res;
    }
}
