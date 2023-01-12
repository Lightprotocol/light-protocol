import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
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
  verifierProgramZeroProgramId,
} from "../index";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { Transaction } from "../transaction";
import { Verifier, PublicInputs } from ".";
import { REGISTERED_VERIFIER_PDA } from "../index";
import {
  VerifierProgramZero,
  VerifierProgramZeroIdl,
} from "../idls/verifier_program_zero";
// TODO: Explore alternative architecture in which verifiers inherit/extend or include
// the Transaction class not the other way around like it is right now
export class VerifierZero implements Verifier {
  verifierProgram: Program<VerifierProgramZeroIdl>;
  wtnsGenPath: String;
  zkeyPath: String;
  calculateWtns: NodeRequire;
  registeredVerifierPda: PublicKey;
  constructor() {
    // Does not work within sdk
    // TODO: bundle files in npm package
    this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
    this.zkeyPath = `./build-circuits/transactionMasp2`;
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.registeredVerifierPda = REGISTERED_VERIFIER_PDA;
  }

  parsePublicInputsFromArray(transaction: Transaction): PublicInputs {
    if (transaction.publicInputsBytes) {
      if (transaction.publicInputsBytes.length == 9) {
        return {
          root: transaction.publicInputsBytes[0],
          publicAmount: transaction.publicInputsBytes[1],
          extDataHash: transaction.publicInputsBytes[2],
          feeAmount: transaction.publicInputsBytes[3],
          mintPubkey: transaction.publicInputsBytes[4],
          nullifiers: [
            transaction.publicInputsBytes[5],
            transaction.publicInputsBytes[6],
          ],
          leaves: [
            [
              transaction.publicInputsBytes[7],
              transaction.publicInputsBytes[8],
            ],
          ],
        };
      } else {
        throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 9`;
      }
    } else {
      throw new Error("public input bytes undefined");
    }
  }
  // TODO: serializeTransaction for relayer

  initVerifierProgram(): void {
    this.verifierProgram = new Program(
      VerifierProgramZero,
      verifierProgramZeroProgramId
    );
  }

  async sendTransaction(transaction: Transaction): Promise<any> {
    this.verifierProgram = new Program(
      VerifierProgramZero,
      verifierProgramZeroProgramId
    );
    console.log("sendTransaction", transaction.recipientFee);

    // await transaction.getPdaAddresses();
    // TODO: move to an init function
    try {
      transaction.recipientBalancePriorTx = (
        await getAccount(
          transaction.provider.connection,
          transaction.recipient,
          TOKEN_PROGRAM_ID
        )
      ).amount;
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        transaction.recipientBalancePriorTx =
          await transaction.provider.connection.getBalance(
            transaction.recipient
          );
      } catch (e) {}
    }
    try {
      transaction.recipientFeeBalancePriorTx =
        await transaction.provider.connection.getBalance(
          transaction.recipientFee
        );
    } catch (error) {
      console.log(
        "transaction.recipientFeeBalancePriorTx fetch failed ",
        transaction.recipientFee
      );
    }

    transaction.senderFeeBalancePriorTx =
      await transaction.provider.connection.getBalance(transaction.senderFee);
    console.log("sendTransaction ");

    transaction.relayerRecipientAccountBalancePriorLastTx =
      await transaction.provider.connection.getBalance(
        transaction.relayerRecipient
      );
    console.log("sendTransaction ");
    // ain derived pda pubkey (Dz5VbR8yVXNg9DsSujFL9mE7U9zTkxBy9NPgH24CEocJ, 254)',
    // 'Program log: Passed-in pda pubkey 7youSP8CcfuvSWxGyJf1JVwaHux6k2Wq15dFPAJUTJvS',
    // 'Program log: Instruction data seed  [32, 221, 13, 181, 197, 157, 122, 91, 137, 50, 220, 253, 86, 14, 185, 235, 248, 65, 247, 142, 135, 232, 230, 228, 140, 194, 44, 128, 82, 67, 8, 147]',
    // "Program log: panicked at 'called `Result::unwrap()` on an `Err` value: InvalidInstructionData', programs/merkle_tree_program/src/verifier_invoked_instructions/insert_nullifier.rs:36:10",
    // 'Program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 consumed 1196752 of 1196752 compute units',
    // 'Program failed to complete: SBF program panicked',

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
    // console.log("senderFee account: ", await transaction.provider.connection.getAccountInfo(transaction.senderFee, "confirmed"));

    // console.log("recipientFee:       ", transaction.recipientFee)
    // console.log("relayerRecipient:   ", transaction.relayerRecipient)
    // console.log("escrow:             ", transaction.escrow)
    // console.log("tokenAuthority:     ", transaction.tokenAuthority)
    // console.log("registeredVerifierPd",transaction.registeredVerifierPda)

    // console.log("transaction.leavesPdaPubkeys ", transaction.leavesPdaPubkeys[0].toBase58());
    // console.log("transaction.signerAuthorityPubkey ", transaction.signerAuthorityPubkey.toBase58());

    const ix = await transaction.verifier.verifierProgram.methods
      .shieldedTransferInputs(
        Buffer.from(transaction.proofBytes),
        Buffer.from(transaction.publicInputs.publicAmount),
        transaction.publicInputs.nullifiers,
        transaction.publicInputs.leaves[0],
        Buffer.from(transaction.publicInputs.feeAmount),
        new anchor.BN(transaction.rootIndex.toString()),
        new anchor.BN(transaction.relayerFee.toString()),
        Buffer.from(transaction.encryptedUtxos.slice(0, 190)) // remaining bytes can be used once tx sizes increase
      )
      .accounts({
        signingAddress: transaction.relayerPubkey,
        systemProgram: SystemProgram.programId,
        programMerkleTree: transaction.merkleTreeProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTree: transaction.merkleTreePubkey,
        preInsertedLeavesIndex: transaction.preInsertedLeavesIndex,
        authority: transaction.signerAuthorityPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
        sender: transaction.sender,
        recipient: transaction.recipient,
        senderFee: transaction.senderFee,
        recipientFee: transaction.recipientFee,
        relayerRecipient: transaction.relayerRecipient,
        escrow: transaction.escrow,
        tokenAuthority: transaction.tokenAuthority,
        registeredVerifierPda: transaction.verifier.registeredVerifierPda,
      })
      .remainingAccounts([
        {
          isSigner: false,
          isWritable: true,
          pubkey: transaction.nullifierPdaPubkeys[0],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transaction.nullifierPdaPubkeys[1],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: transaction.leavesPdaPubkeys[0],
        },
      ])
      .signers([transaction.payer])
      .instruction();

    let recentBlockhash = (
      await transaction.provider.connection.getRecentBlockhash("confirmed")
    ).blockhash;
    let txMsg = new TransactionMessage({
      payerKey: transaction.payer.publicKey,
      instructions: [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ix,
      ],
      recentBlockhash: recentBlockhash,
    });

    let lookupTableAccount =
      await transaction.provider.connection.getAccountInfo(
        transaction.lookupTable,
        "confirmed"
      );

    let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
      lookupTableAccount.data
    );

    let compiledTx = txMsg.compileToV0Message([
      { state: unpackedLookupTableAccount },
    ]);
    compiledTx.addressTableLookups[0].accountKey = transaction.lookupTable;

    let tx = new VersionedTransaction(compiledTx);
    let retries = 3;
    let res;
    while (retries > 0) {
      tx.sign([transaction.payer]);
      recentBlockhash = (
        await transaction.provider.connection.getRecentBlockhash("confirmed")
      ).blockhash;

      try {
        let serializedTx = tx.serialize();
        console.log("serializedTx: ");

        res = await sendAndConfirmRawTransaction(
          transaction.provider.connection,
          serializedTx,
          confirmConfig
        );
        retries = 0;
        console.log(res);
      } catch (e) {
        retries--;
        if (retries == 0 || e.logs != undefined) {
          console.log(e);
          return e;
        }
      }
    }
    return res;
  }
}
