import {
  VerifierProgramOne,
  VerifierProgramOneIdl,
} from "../idls/verifier_program_one";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  TransactionMessage,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  VersionedTransaction,
  sendAndConfirmRawTransaction,
} from "@solana/web3.js";
import {
  DEFAULT_PROGRAMS,
  REGISTERED_VERIFIER_ONE_PDA,
  verifierProgramOneProgramId,
} from "../index";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { assert } from "chai";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { verifierProgramOne } from "../index";

export class VerifierOne implements Verifier {
  verifierProgram: Program<VerifierProgramOneIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor() {
    this.verifierProgram = new Program(
      VerifierProgramOne,
      verifierProgramOneProgramId
    );
    this.wtnsGenPath =
      "./build-circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = "./build-circuits/transactionMasp10";
    this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
    this.registeredVerifierPda = REGISTERED_VERIFIER_ONE_PDA;
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {
    if (transaction.publicInputsBytes.length == 17) {
      return {
        root: transaction.publicInputsBytes[0],
        publicAmount: transaction.publicInputsBytes[1],
        extDataHash: transaction.publicInputsBytes[2],
        feeAmount: transaction.publicInputsBytes[3],
        mintPubkey: transaction.publicInputsBytes[4],
        nullifiers: Array.from(transaction.publicInputsBytes.slice(5, 15)),
        leaves: [
          [
            transaction.publicInputsBytes[15],
            transaction.publicInputsBytes[16],
          ],
        ],
      };
    } else {
      throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 17`;
    }
  }

  initVerifierProgram(): void {
    this.verifierProgram = new Program(
      VerifierProgramOne,
      verifierProgramOneProgramId
    );
  }

  async transferFirst(transfer: Transaction) {
    console.log("in transferFirst");

    console.log(
      "transfer.publicInputs.publicAmount: ",
      transfer.publicInputs.publicAmount
    );
    console.log(
      "transfer.publicInputs.nullifiers: ",
      transfer.publicInputs.nullifiers
    );
    console.log(
      "transfer.publicInputs.leaves[0]: ",
      transfer.publicInputs.leaves[0]
    );
    console.log(
      "transfer.publicInputs.feeAmount: ",
      transfer.publicInputs.feeAmount
    );

    console.log("transfer.rootIndex.toString(): ", transfer.rootIndex);

    console.log(
      "transfer.publicInputs.feeAmount: ",
      transfer.publicInputs.feeAmount
    );
    console.log(
      "transfer.publicInputs.relayerFee.toString(): ",
      transfer.relayerFee.toString()
    );
    console.log(
      "transfer.publicInputs.encryptedUtxos: ",
      transfer.encryptedUtxos
    );
    console.log("transfer.relayerPubkey: ", transfer.relayerPubkey);
    console.log("transfer.verifierStatePubkey: ", transfer.verifierStatePubkey);
    console.log("transfer.payer: ", transfer.payer);
    console.log(
      "transfer.signerAuthorityPubkey: ",
      transfer.signerAuthorityPubkey.toBase58()
    );
    console.log(
      "transfer.tokenAuthority: ",
      transfer.tokenAuthority.toBase58()
    );
    console.log("transfer.senderFee: ", transfer.senderFee.toBase58());
    console.log("transfer.recipientFee: ", transfer.recipientFee.toBase58());
    console.log(
      "transfer.relayerRecipient: ",
      transfer.relayerRecipient.toBase58()
    );
    console.log("transfer.escrow: ", transfer.escrow.toBase58());
    console.log(
      "transfer.registeredVerifierPda: ",
      transfer.verifier.registeredVerifierPda.toBase58()
    );

    const ix1 = await transfer.verifier.verifierProgram.methods
      .shieldedTransferFirst(
        Buffer.from(transfer.publicInputs.publicAmount),
        transfer.publicInputs.nullifiers,
        transfer.publicInputs.leaves[0],
        Buffer.from(transfer.publicInputs.feeAmount),
        new anchor.BN(transfer.rootIndex.toString()),
        new anchor.BN(transfer.relayerFee.toString()),
        Buffer.from(transfer.encryptedUtxos)
      )
      .accounts({
        signingAddress: transfer.relayerPubkey,
        systemProgram: SystemProgram.programId,
        verifierState: transfer.verifierStatePubkey,
      })
      .signers([transfer.payer])
      .rpc({
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      });
    console.log("ix1 success ", ix1);
  }

  async transferSecond(transfer: Transaction) {
    const ix = await transfer.verifier.verifierProgram.methods
      .shieldedTransferSecond(Buffer.from(transfer.proofBytes))
      .accounts({
        signingAddress: transfer.relayerPubkey,
        verifierState: transfer.verifierStatePubkey,
        systemProgram: SystemProgram.programId,
        programMerkleTree: transfer.merkleTreeProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTree: transfer.merkleTreePubkey,
        preInsertedLeavesIndex: transfer.preInsertedLeavesIndex,
        authority: transfer.signerAuthorityPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
        sender: transfer.sender,
        recipient: transfer.recipient,
        senderFee: transfer.senderFee,
        recipientFee: transfer.recipientFee,
        relayerRecipient: transfer.relayerRecipient,
        escrow: transfer.escrow,
        tokenAuthority: transfer.tokenAuthority,
        registeredVerifierPda: transfer.verifier.registeredVerifierPda,
      })
      .remainingAccounts([
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[0],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[1],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[2],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[3],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[4],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[5],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[6],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[7],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[8],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.nullifierPdaPubkeys[9],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transfer.leavesPdaPubkeys[0],
        },
      ])
      .signers([transfer.payer])
      .instruction();
    let recentBlockhash = (
      await transfer.provider.connection.getRecentBlockhash("confirmed")
    ).blockhash;

    let txMsg = new TransactionMessage({
      payerKey: transfer.payer.publicKey,
      instructions: [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ix,
      ],
      recentBlockhash: recentBlockhash,
    });

    let lookupTableAccount = await transfer.provider.connection.getAccountInfo(
      transfer.lookupTable,
      "confirmed"
    );

    let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
      lookupTableAccount.data
    );

    let compiledTx = txMsg.compileToV0Message([
      { state: unpackedLookupTableAccount },
    ]);
    compiledTx.addressTableLookups[0].accountKey = transfer.lookupTable;

    let transaction = new VersionedTransaction(compiledTx);
    let retries = 3;
    let res;
    while (retries > 0) {
      transaction.sign([transfer.payer]);
      recentBlockhash = (
        await transfer.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;
      transaction.message.recentBlockhash = recentBlockhash;
      let serializedTx = transaction.serialize();

      try {
        console.log("serializedTx: ");

        res = await sendAndConfirmRawTransaction(
          transfer.provider.connection,
          serializedTx,
          {
            commitment: "confirmed",
            preflightCommitment: "confirmed",
          }
        );
        retries = 0;
      } catch (e) {
        console.log(e);
        retries--;
        if (retries == 0 || e.logs != undefined) {
          const ixClose = await transfer.verifier.verifierProgram.methods
            .closeVerifierState()
            .accounts({
              signingAddress: transfer.relayerPubkey,
              verifierState: transfer.verifierStatePubkey,
            })
            .signers([transfer.payer])
            .rpc({
              commitment: "confirmed",
              preflightCommitment: "confirmed",
            });
          return e;
        }
      }
    }
  }

  async sendTransaction(transfer: Transaction): Promise<any> {
    assert(transfer.nullifierPdaPubkeys.length == 10);
    let balance = await transfer.provider.connection.getBalance(
      transfer.signerAuthorityPubkey,
      { preflightCommitment: "confirmed", commitment: "confirmed" }
    );
    if (balance === 0) {
      await transfer.provider.connection.confirmTransaction(
        await transfer.provider.connection.requestAirdrop(
          transfer.signerAuthorityPubkey,
          1_000_000_000
        ),
        { preflightCommitment: "confirmed", commitment: "confirmed" }
      );
    }
    try {
      transfer.recipientBalancePriorTx = (
        await getAccount(
          transfer.provider.connection,
          transfer.recipient,
          TOKEN_PROGRAM_ID
        )
      ).amount;
    } catch (error) {}
    transfer.recipientFeeBalancePriorTx =
      await transfer.provider.connection.getBalance(transfer.recipientFee);
    // console.log("recipientBalancePriorTx: ", transfer.recipientBalancePriorTx);
    // console.log("recipientFeeBalancePriorTx: ", transfer.recipientFeeBalancePriorTx);
    // console.log("sender_fee: ", transfer.senderFee);
    transfer.senderFeeBalancePriorTx =
      await transfer.provider.connection.getBalance(transfer.senderFee);
    transfer.relayerRecipientAccountBalancePriorLastTx =
      await transfer.provider.connection.getBalance(transfer.relayerRecipient);

    // console.log("signingAddress:     ", transfer.relayerPubkey)
    // console.log("systemProgram:      ", SystemProgram.programId)
    // console.log("programMerkleTree:  ", transfer.merkleTreeProgram.programId)
    // console.log("rent:               ", DEFAULT_PROGRAMS.rent)
    // console.log("merkleTree:         ", transfer.merkleTreePubkey)
    // console.log("preInsertedLeavesInd", transfer.preInsertedLeavesIndex)
    // console.log("authority:          ", transfer.signerAuthorityPubkey)
    // console.log("tokenProgram:       ", TOKEN_PROGRAM_ID)
    // console.log("sender:             ", transfer.sender)
    // console.log("recipient:          ", transfer.recipient)
    // console.log("senderFee:          ", transfer.senderFee)
    // console.log("recipientFee:       ", transfer.recipientFee)
    // console.log("relayerRecipient:   ", transfer.relayerRecipient)
    // console.log("escrow:             ", transfer.escrow)
    // console.log("tokenAuthority:     ", transfer.tokenAuthority)
    // console.log("registeredVerifierPd",transfer.registeredVerifierPda)
    // console.log("encryptedUtxos len ", transfer.encryptedUtxos.length);
    // console.log("transfer.encryptedUtxos[0], ", transfer.encryptedUtxos);
    console.log(
      "transfer.verifierStatePubkey, ",
      transfer.verifierStatePubkey.toBase58()
    );
    // console.log("transfer.publicInputs.nullifiers, ", transfer.publicInputs.nullifiers);
    // console.log("transfer.rootIndex ", transfer.rootIndex);
    // console.log("transfer.relayerFee ", transfer.relayerFee);
    // console.log("transfer.encryptedUtxos ", transfer.encryptedUtxos);
    // transfer.transferFirst = transferFirst;
    // transfer.transferSecond = transferSecond;

    //TODO: think about how to do transfer in a better way transfer is quite confusing since the transfer in transfer fn is not shieldedTransfer not the verifier object
    let res = await transfer.verifier.transferFirst(transfer);
    res = await transfer.verifier.transferSecond(transfer);

    return res;
  }
}
