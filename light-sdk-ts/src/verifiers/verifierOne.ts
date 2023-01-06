import { VerifierProgramOne } from "../../idls/verifier_program_one";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {Connection, PublicKey, Keypair, SystemProgram, TransactionMessage, ComputeBudgetProgram,  AddressLookupTableAccount, VersionedTransaction, sendAndConfirmRawTransaction } from "@solana/web3.js";
import {
  DEFAULT_PROGRAMS, REGISTERED_VERIFIER_ONE_PDA,
} from "../constants";
import { TOKEN_PROGRAM_ID, getAccount  } from '@solana/spl-token';
import { assert } from "chai";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import {verifierProgramOne } from "../constants"

export class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOne>
  wtnsGenPath: String
  zkeyPath: String
  calculateWtns: NodeRequire
  registeredVerifierPda: PublicKey
  constructor() {
    this.verifierProgram = verifierProgramOne;
    this.wtnsGenPath = "./build-circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = './build-circuits/transactionMasp10'
    this.calculateWtns = require('../../build-circuits/transactionMasp10_js/witness_calculator.js')
    this.registeredVerifierPda =  REGISTERED_VERIFIER_ONE_PDA

  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {

    if (transaction.publicInputsBytes.length == 17) {
        return {
         root:         transaction.publicInputsBytes[0],
         publicAmount: transaction.publicInputsBytes[1],
         extDataHash:  transaction.publicInputsBytes[2],
         feeAmount:    transaction.publicInputsBytes[3],
         mintPubkey:   transaction.publicInputsBytes[4],
         nullifiers:   Array.from(transaction.publicInputsBytes.slice(5,15)),
         leaves:     [[transaction.publicInputsBytes[15], transaction.publicInputsBytes[16]]]
       };
    } else {
      throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 17`;
    }

  }

  async transferFirst(transfer: Transaction) {
    console.log("in transferFirst");


    const ix1 = await transfer.verifier.verifierProgram.methods.shieldedTransferFirst(
      Buffer.from(transfer.publicInputs.publicAmount),
      transfer.publicInputs.nullifiers,
      transfer.publicInputs.leaves[0],
      Buffer.from(transfer.publicInputs.feeAmount),
      new anchor.BN(transfer.rootIndex.toString()),
      new anchor.BN(transfer.relayerFee.toString()),
      Buffer.from(transfer.encryptedUtxos)
    ).accounts(
      {
        signingAddress:     transfer.relayerPubkey,
        systemProgram:      SystemProgram.programId,
        verifierState:      transfer.verifierStatePubkey
      }
    )
    .signers([transfer.payer])
    .rpc({
      commitment: 'confirmed',
      preflightCommitment: 'confirmed',
    });
    console.log("ix1 success ", ix1);
  }

  async transferSecond(transfer: Transaction) {
    const ix = await transfer.verifier.verifierProgram.methods.shieldedTransferSecond(
      Buffer.from(transfer.proofBytes)
    ).accounts(
      {
        signingAddress:     transfer.relayerPubkey,
        verifierState:      transfer.verifierStatePubkey,
        systemProgram:      SystemProgram.programId,
        programMerkleTree:  transfer.merkleTreeProgram.programId,
        rent:               DEFAULT_PROGRAMS.rent,
        merkleTree:         transfer.merkleTreePubkey,
        preInsertedLeavesIndex: transfer.preInsertedLeavesIndex,
        authority:          transfer.signerAuthorityPubkey,
        tokenProgram:       TOKEN_PROGRAM_ID,
        sender:             transfer.sender,
        recipient:          transfer.recipient,
        senderFee:          transfer.senderFee,
        recipientFee:       transfer.recipientFee,
        relayerRecipient:   transfer.relayerRecipient,
        escrow:             transfer.escrow,
        tokenAuthority:     transfer.tokenAuthority,
        registeredVerifierPda: transfer.verifier.registeredVerifierPda
      }
    )
    .remainingAccounts([
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[0]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[1]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[2]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[3]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[4]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[5]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[6]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[7]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[8]},
      { isSigner: false, isWritable: true, pubkey: transfer.nullifierPdaPubkeys[9]},
      { isSigner: false, isWritable: true, pubkey: transfer.leavesPdaPubkeys[0]}
    ])
    .signers([transfer.payer]).instruction();
    let recentBlockhash = (await transfer.provider.connection.getRecentBlockhash("confirmed")).blockhash;


    let txMsg = new TransactionMessage({
      payerKey: transfer.payer.publicKey,
      instructions: [
        ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
        ix
      ],
      recentBlockhash: recentBlockhash})

      let lookupTableAccount = await transfer.provider.connection.getAccountInfo(transfer.lookupTable, "confirmed");

      let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(lookupTableAccount.data);

      let compiledTx = txMsg.compileToV0Message([{state: unpackedLookupTableAccount}]);
      compiledTx.addressTableLookups[0].accountKey = transfer.lookupTable

      let transaction = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res
      while (retries > 0) {
        transaction.sign([transfer.payer])
        recentBlockhash = (await transfer.provider.connection.getRecentBlockhash("confirmed")).blockhash;
        transaction.message.recentBlockhash = recentBlockhash;
        let serializedTx = transaction.serialize();

        try {
          console.log("serializedTx: ");

          res = await sendAndConfirmRawTransaction(transfer.provider.connection, serializedTx,
            {
              commitment: 'confirmed',
              preflightCommitment: 'confirmed',
            }
          );
          retries = 0;

        } catch (e) {
          console.log(e);
          retries--;
          if (retries == 0 || e.logs != undefined) {
            const ixClose = await transfer.verifier.verifierProgram.methods.closeVerifierState(
            ).accounts(
              {
                signingAddress:     transfer.relayerPubkey,
                verifierState:      transfer.verifierStatePubkey
              }
            )
            .signers([transfer.payer]).rpc({
              commitment: 'confirmed',
              preflightCommitment: 'confirmed',
            });
            return e;
          }
        }

      }
    }

  async sendTransaction(insert: Boolean = true): Promise<any> {
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
      // console.log("encryptedUtxos len ", this.encryptedUtxos.length);
      // console.log("this.encryptedUtxos[0], ", this.encryptedUtxos);
      console.log("this.verifierStatePubkey, ", this.verifierStatePubkey.toBase58());
      // console.log("this.publicInputs.nullifiers, ", this.publicInputs.nullifiers);
      // console.log("this.rootIndex ", this.rootIndex);
      // console.log("this.relayerFee ", this.relayerFee);
      // console.log("this.encryptedUtxos ", this.encryptedUtxos);
      // this.transferFirst = transferFirst;
      // this.transferSecond = transferSecond;

      //TODO: think about how to do this in a better way this is quite confusing since the this in this fn is not shieldedTransfer not the verifier object
      let res = await this.verifier.transferFirst(this);
      res = await this.verifier.transferSecond(this);

      return res;
    }
}
