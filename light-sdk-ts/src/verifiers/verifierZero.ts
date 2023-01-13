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
  AUTHORITY,
  confirmConfig,
  DEFAULT_PROGRAMS,
  MERKLE_TREE_KEY,
  PRE_INSERTED_LEAVES_INDEX,
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
  signerAuthorityPda?: PublicKey;
  config: { in: number; out: number };
  constructor() {
    // Proofgen does not work within sdk needs circuit-build
    // TODO: bundle files in npm package
    this.verifierProgram = new Program(
      VerifierProgramZero,
      verifierProgramZeroProgramId
    );
    this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
    this.zkeyPath = `./build-circuits/transactionMasp2`;
    this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
    this.registeredVerifierPda = REGISTERED_VERIFIER_PDA;
    this.config = { in: 2, out: 2 };
  }

  getSignerAuthorityPda(merkleTreeProgramId: PublicKey) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      this.verifierProgram.programId
    )[0];
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
    // TODO: move to an init function
    // await transaction.getPdaAddresses();
    try {
      transaction.recipientBalancePriorTx = (
        await getAccount(
          transaction.instance.provider.connection,
          transaction.params.accounts.recipient,
          TOKEN_PROGRAM_ID
        )
      ).amount;
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        transaction.recipientBalancePriorTx =
          await transaction.instance.provider.connection.getBalance(
            transaction.params.accounts.recipient
          );
      } catch (e) {}
    }
    try {
      transaction.recipientFeeBalancePriorTx =
        await transaction.instance.provider.connection.getBalance(
          transaction.params.accounts.recipientFee
        );
    } catch (error) {
      console.log(
        "transaction.recipientFeeBalancePriorTx fetch failed ",
        transaction.params.accounts.recipientFee
      );
    }

    transaction.senderFeeBalancePriorTx =
      await transaction.instance.provider.connection.getBalance(
        transaction.params.accounts.senderFee
      );

    transaction.relayerRecipientAccountBalancePriorLastTx =
      await transaction.instance.provider.connection.getBalance(
        transaction.relayer.accounts.relayerRecipient
      );

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
    // console.log("senderFee account: ", await transaction.instance.provider.connection.getAccountInfo(transaction.senderFee, "confirmed"));

    // console.log("recipientFee:       ", transaction.recipientFee)
    // console.log("relayerRecipient:   ", transaction.relayerRecipient)
    // console.log("escrow:             ", transaction.escrow)
    // console.log("tokenAuthority:     ", transaction.tokenAuthority)
    // console.log("registeredVerifierPd",transaction.verifier.registeredVerifierPda)

    // console.log("transaction.leavesPdaPubkeys ", transaction.leavesPdaPubkeys[0].toBase58());
    // console.log("transaction.signerAuthorityPubkey ", transaction.signerAuthorityPubkey.toBase58());

    if (
      transaction.params &&
      transaction.params.nullifierPdaPubkeys &&
      transaction.params.leavesPdaPubkeys
    ) {
      if (!transaction.payer) {
        throw new Error("Payer not defined");
      }
      this.initVerifierProgram();
      const ix = await this.verifierProgram.methods
        .shieldedTransferInputs(
          Buffer.from(transaction.proofBytes),
          Buffer.from(transaction.publicInputs.publicAmount),
          transaction.publicInputs.nullifiers,
          transaction.publicInputs.leaves[0],
          Buffer.from(transaction.publicInputs.feeAmount),
          new anchor.BN(transaction.rootIndex.toString()),
          new anchor.BN(transaction.relayer.relayerFee.toString()),
          Buffer.from(transaction.encryptedUtxos.slice(0, 190)) // remaining bytes can be used once tx sizes increase
        )
        .accounts({
          ...transaction.params.accounts,
          ...transaction.relayer.accounts,
        })
        .remainingAccounts([
          ...transaction.params.nullifierPdaPubkeys,
          ...transaction.params.leavesPdaPubkeys,
        ])
        .signers([transaction.payer])
        .instruction();

      const recentBlockhash = (
        await transaction.instance.provider.connection.getRecentBlockhash(
          "confirmed"
        )
      ).blockhash;
      const txMsg = new TransactionMessage({
        payerKey: transaction.payer.publicKey,
        instructions: [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ix,
        ],
        recentBlockhash: recentBlockhash,
      });

      const lookupTableAccount =
        await transaction.instance.provider.connection.getAccountInfo(
          transaction.relayer.accounts.lookUpTable,
          "confirmed"
        );

      const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
        lookupTableAccount.data
      );

      const compiledTx = txMsg.compileToV0Message([
        {
          state: unpackedLookupTableAccount,
          key: transaction.relayer.accounts.lookUpTable,
          isActive: () => {
            return true;
          },
        },
      ]);

      compiledTx.addressTableLookups[0].accountKey =
        transaction.relayer.accounts.lookUpTable;

      const tx = new VersionedTransaction(compiledTx);
      let retries = 3;
      let res;
      while (retries > 0) {
        tx.sign([transaction.payer]);
        // recentBlockhash = (
        //   await transaction.instance.provider.connection.getRecentBlockhash("confirmed")
        // ).blockhash;

        try {
          let serializedTx = tx.serialize();
          console.log("serializedTx: ");

          res = await sendAndConfirmRawTransaction(
            transaction.instance.provider.connection,
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
}
