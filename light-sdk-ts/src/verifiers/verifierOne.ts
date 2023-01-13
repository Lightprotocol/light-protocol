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
  confirmConfig,
  DEFAULT_PROGRAMS,
  MERKLE_TREE_KEY,
  PRE_INSERTED_LEAVES_INDEX,
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
  config: { in: number; out: number };
  signerAuthorityPda: PublicKey;

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
    this.config = { in: 10, out: 2 };
  }

  getSignerAuthorityPda(merkleTreeProgramId: PublicKey) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      this.verifierProgram.programId
    )[0];
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

  async transferFirst(transaction: Transaction) {
    console.log("in transferFirst");

    // console.log(
    //   "transaction.publicInputs.publicAmount: ",
    //   transaction.publicInputs.publicAmount
    // );
    // console.log(
    //   "transaction.publicInputs.nullifiers: ",
    //   transaction.publicInputs.nullifiers
    // );
    // console.log(
    //   "transaction.publicInputs.leaves[0]: ",
    //   transaction.publicInputs.leaves[0]
    // );
    // console.log(
    //   "transaction.publicInputs.feeAmount: ",
    //   transaction.publicInputs.feeAmount
    // );

    // console.log("transaction.rootIndex.toString(): ", transaction.rootIndex);

    // console.log(
    //   "transaction.publicInputs.feeAmount: ",
    //   transaction.publicInputs.feeAmount
    // );
    // console.log(
    //   "transaction.publicInputs.relayerFee.toString(): ",
    //   transaction.relayerFee.toString()
    // );
    // console.log(
    //   "transaction.publicInputs.encryptedUtxos: ",
    //   transaction.encryptedUtxos
    // );
    // console.log("transaction.relayerPubkey: ", transaction.relayerPubkey);
    // console.log("transaction.verifierState: ", transaction.verifierState);
    // console.log("transaction.payer: ", transaction.payer);
    // console.log(
    //   "transaction.signerAuthorityPubkey: ",
    //   transaction.signerAuthorityPubkey.toBase58()
    // );
    // console.log(
    //   "transaction.tokenAuthority: ",
    //   transaction.tokenAuthority.toBase58()
    // );
    // console.log("transaction.senderFee: ", transaction.senderFee.toBase58());
    // console.log("transaction.recipientFee: ", transaction.recipientFee.toBase58());
    // console.log(
    //   "transaction.relayerRecipient: ",
    //   transaction.relayerRecipient.toBase58()
    // );
    // console.log("transaction.escrow: ", transaction.escrow.toBase58());
    // console.log(
    //   "transaction.registeredVerifierPda: ",
    //   transaction.verifier.registeredVerifierPda.toBase58()
    // );

    const ix1 = await this.verifierProgram.methods
      .shieldedTransferFirst(
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves[0],
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()),
        new anchor.BN(transaction.relayer.relayerFee.toString()),
        Buffer.from(transaction.encryptedUtxos)
      )
      .accounts({
        signingAddress: transaction.relayer.relayerPubkey,
        systemProgram: SystemProgram.programId,
        verifierState: transaction.params?.accounts.verifierState,
      })
      .signers([transaction.payer])
      .rpc(confirmConfig);
    console.log("ix1 success ", ix1);
  }

  async transferSecond(transaction: Transaction) {
    const ix = await this.verifierProgram.methods
      .shieldedTransferSecond(Buffer.from(transaction.proofBytes))
      .accounts({
        signingAddress: transaction.relayer.relayerPubkey,
        systemProgram: SystemProgram.programId,
        programMerkleTree: transaction.merkleTreeProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        authority: this.getSignerAuthorityPda(
          transaction.merkleTreeProgram.programId),
        tokenProgram: TOKEN_PROGRAM_ID,
        // sender: transaction.sender,
        // recipient: transaction.recipient,
        // senderFee: transaction.senderFee,
        // recipientFee: transaction.recipientFee,
        relayerRecipient: transaction.relayer.relayerRecipient,
        // escrow: transaction.escrow,
        // tokenAuthority: transaction.tokenAuthority,
        registeredVerifierPda: this.registeredVerifierPda,
        ...transaction.params.accounts,
      })
      .remainingAccounts([
        ...transaction.params.nullifierPdaPubkeys,
        ...transaction.params.leavesPdaPubkeys,
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[0],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[1],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[2],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[3],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[4],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[5],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[6],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[7],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[8],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.nullifierPdaPubkeys[9],
        // },
        // {
        //   isSigner: false,
        //   isWritable: true,
        //   pubkey: transaction.leavesPdaPubkeys[0],
        // },
      ])
      .signers([transaction.payer])
      .instruction();
    let recentBlockhash = (
      await transaction.instance.provider.connection.getRecentBlockhash("confirmed")
    ).blockhash;

    let txMsg = new TransactionMessage({
      payerKey: transaction.payer.publicKey,
      instructions: [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ix,
      ],
      recentBlockhash: recentBlockhash,
    });

    let lookupTableAccount = await transaction.instance.provider.connection.getAccountInfo(
      transaction.relayer.lookUpTable,
      "confirmed"
    );

    let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
      lookupTableAccount.data
    );

    let compiledTx = txMsg.compileToV0Message([
      { state: unpackedLookupTableAccount },
    ]);
    compiledTx.addressTableLookups[0].accountKey = transaction.relayer.lookUpTable;

    let tx = new VersionedTransaction(compiledTx);
    let retries = 3;
    let res;
    while (retries > 0) {
      tx.sign([transaction.payer]);
      recentBlockhash = (
        await transaction.instance.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;
      tx.message.recentBlockhash = recentBlockhash;
      let serializedTx = tx.serialize();

      try {
        console.log("serializedTx: ");

        res = await sendAndConfirmRawTransaction(
          transaction.instance.provider.connection,
          serializedTx,
          confirmConfig
        );
        retries = 0;
      } catch (e) {
        console.log(e);
        retries--;
        if (retries == 0 || e.logs != undefined) {
          const ixClose = await this.verifierProgram.methods
            .closeVerifierState()
            .accounts({
              signingAddress: transaction.relayer.relayerPubkey,
              verifierState: transaction.params?.accounts.verifierState,
            })
            .signers([transaction.payer])
            .rpc(confirmConfig);
          return e;
        }
      }
    }
  }

  async sendTransaction(transaction: Transaction): Promise<any> {
    if(transaction && transaction.params && transaction.instance.provider) {
      assert(transaction.params?.nullifierPdaPubkeys?.length == 10);
      let balance = await transaction.instance.provider.connection.getBalance(
        this.getSignerAuthorityPda(transaction.merkleTreeProgram?.programId),
        "confirmed"
      );
      if (balance === 0) {
        await transaction.instance.provider.connection.confirmTransaction(
          await transaction.instance.provider.connection.requestAirdrop(
            this.getSignerAuthorityPda(transaction.merkleTreeProgram?.programId),
            1_000_000_000
          ),
          "confirmed"
        );
      }
      try {
        transaction.recipientBalancePriorTx = new anchor.BN((
          await getAccount(
            transaction.instance.provider.connection,
            transaction.params.accounts.recipient,
            TOKEN_PROGRAM_ID
          )
        ).amount.toString());
      } catch (error) {}
      console.log("here");
      
      transaction.recipientFeeBalancePriorTx =
        await transaction.instance.provider.connection.getBalance(transaction.params.accounts.recipientFee);
      // console.log("recipientBalancePriorTx: ", transaction.recipientBalancePriorTx);
      // console.log("recipientFeeBalancePriorTx: ", transaction.recipientFeeBalancePriorTx);
      // console.log("sender_fee: ", transaction.senderFee);
      console.log("here1");
      transaction.senderFeeBalancePriorTx =
        await transaction.instance.provider.connection.getBalance(transaction.params?.accounts.senderFee);
        console.log("here2");
      transaction.relayerRecipientAccountBalancePriorLastTx =
        await transaction.instance.provider.connection.getBalance(transaction.relayer.relayerRecipient);
  
      // console.log("signingAddress:     ", transaction.relayerPubkey)
      // console.log("systemProgram:      ", SystemProgram.programId)
      // console.log("programMerkleTree:  ", transaction.merkleTreeProgram.programId)
      // console.log("rent:               ", DEFAULT_PROGRAMS.rent)
      // console.log("merkleTree:         ", transaction.merkleTreePubkey)
      // console.log("preInsertedLeavesInd", transaction.preInsertedLeavesIndex)
      // console.log("authority:          ", transaction.signerAuthorityPubkey)
      // console.log("tokenProgram:       ", TOKEN_PROGRAM_ID)
      // console.log("sender:             ", transaction.sender)
      // console.log("recipient:          ", transaction.recipient)
      // console.log("senderFee:          ", transaction.senderFee)
      // console.log("recipientFee:       ", transaction.recipientFee)
      // console.log("relayerRecipient:   ", transaction.relayerRecipient)
      // console.log("escrow:             ", transaction.escrow)
      // console.log("tokenAuthority:     ", transaction.tokenAuthority)
      // console.log("registeredVerifierPd",transaction.registeredVerifierPda)
      // console.log("encryptedUtxos len ", transaction.encryptedUtxos.length);
      // console.log("transaction.encryptedUtxos[0], ", transaction.encryptedUtxos);
      console.log(
        "transaction.verifierState, ",
        transaction.params?.accounts.verifierState.toBase58()
      );
      // console.log("transaction.publicInputs.nullifiers, ", transaction.publicInputs.nullifiers);
      // console.log("transaction.rootIndex ", transaction.rootIndex);
      // console.log("transaction.relayerFee ", transaction.relayerFee);
      // console.log("transaction.encryptedUtxos ", transaction.encryptedUtxos);
      // transaction.transferFirst = transferFirst;
      // transaction.transferSecond = transferSecond;
  
      //TODO: think about how to do transfer in a better way transfer is quite confusing since the transfer in transfer fn is not shieldedTransfer not the verifier object
      let res = await this.transferFirst(transaction);
      res = await this.transferSecond(transaction);
  
      return res;
    }
 
  }
}
