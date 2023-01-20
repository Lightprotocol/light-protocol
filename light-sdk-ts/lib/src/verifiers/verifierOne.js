"use strict";
var __createBinding =
  (this && this.__createBinding) ||
  (Object.create
    ? function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        var desc = Object.getOwnPropertyDescriptor(m, k);
        if (
          !desc ||
          ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)
        ) {
          desc = {
            enumerable: true,
            get: function () {
              return m[k];
            },
          };
        }
        Object.defineProperty(o, k2, desc);
      }
    : function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        o[k2] = m[k];
      });
var __setModuleDefault =
  (this && this.__setModuleDefault) ||
  (Object.create
    ? function (o, v) {
        Object.defineProperty(o, "default", { enumerable: true, value: v });
      }
    : function (o, v) {
        o["default"] = v;
      });
var __importStar =
  (this && this.__importStar) ||
  function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null)
      for (var k in mod)
        if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k))
          __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
  };
var __awaiter =
  (this && this.__awaiter) ||
  function (thisArg, _arguments, P, generator) {
    function adopt(value) {
      return value instanceof P
        ? value
        : new P(function (resolve) {
            resolve(value);
          });
    }
    return new (P || (P = Promise))(function (resolve, reject) {
      function fulfilled(value) {
        try {
          step(generator.next(value));
        } catch (e) {
          reject(e);
        }
      }
      function rejected(value) {
        try {
          step(generator["throw"](value));
        } catch (e) {
          reject(e);
        }
      }
      function step(result) {
        result.done
          ? resolve(result.value)
          : adopt(result.value).then(fulfilled, rejected);
      }
      step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.VerifierOne = void 0;
const anchor = __importStar(require("@project-serum/anchor"));
const web3_js_1 = require("@solana/web3.js");
const constants_1 = require("../constants");
const spl_token_1 = require("@solana/spl-token");
const chai_1 = require("chai");
const constants_2 = require("../constants");
class VerifierOne {
  constructor() {
    this.verifierProgram = constants_2.verifierProgramOne;
    this.wtnsGenPath =
      "./build-circuits/transactionMasp10_js/transactionMasp10";
    this.zkeyPath = "./build-circuits/transactionMasp10";
    this.calculateWtns = require("../../build-circuits/transactionMasp10_js/witness_calculator.js");
    this.registeredVerifierPda = constants_1.REGISTERED_VERIFIER_ONE_PDA;
  }
  parsePublicInputsFromArray(transaction) {
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
  transferFirst(transfer) {
    return __awaiter(this, void 0, void 0, function* () {
      console.log("in transferFirst");
      const ix1 = yield transfer.verifier.verifierProgram.methods
        .shieldedTransferFirst(
          Buffer.from(transfer.publicInputs.publicAmount),
          transfer.publicInputs.nullifiers,
          transfer.publicInputs.leaves[0],
          Buffer.from(transfer.publicInputs.feeAmount),
          new anchor.BN(transfer.rootIndex.toString()),
          new anchor.BN(transfer.relayerFee.toString()),
          Buffer.from(transfer.encryptedUtxos),
        )
        .accounts({
          signingAddress: transfer.relayerPubkey,
          systemProgram: web3_js_1.SystemProgram.programId,
          verifierState: transfer.verifierStatePubkey,
        })
        .signers([transfer.payer])
        .rpc({
          commitment: "confirmed",
          preflightCommitment: "confirmed",
        });
      console.log("ix1 success ", ix1);
    });
  }
  transferSecond(transfer) {
    return __awaiter(this, void 0, void 0, function* () {
      const ix = yield transfer.verifier.verifierProgram.methods
        .shieldedTransferSecond(Buffer.from(transfer.proofBytes))
        .accounts({
          signingAddress: transfer.relayerPubkey,
          verifierState: transfer.verifierStatePubkey,
          systemProgram: web3_js_1.SystemProgram.programId,
          programMerkleTree: transfer.merkleTreeProgram.programId,
          rent: constants_1.DEFAULT_PROGRAMS.rent,
          merkleTree: transfer.merkleTreePubkey,
          preInsertedLeavesIndex: transfer.preInsertedLeavesIndex,
          authority: transfer.signerAuthorityPubkey,
          tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
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
      let recentBlockhash =
        (yield transfer.provider.connection.getRecentBlockhash("confirmed"))
          .blockhash;
      let txMsg = new web3_js_1.TransactionMessage({
        payerKey: transfer.payer.publicKey,
        instructions: [
          web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({
            units: 1400000,
          }),
          ix,
        ],
        recentBlockhash: recentBlockhash,
      });
      let lookupTableAccount =
        yield transfer.provider.connection.getAccountInfo(
          transfer.lookupTable,
          "confirmed",
        );
      let unpackedLookupTableAccount =
        web3_js_1.AddressLookupTableAccount.deserialize(
          lookupTableAccount.data,
        );
      let compiledTx = txMsg.compileToV0Message([
        { state: unpackedLookupTableAccount },
      ]);
      compiledTx.addressTableLookups[0].accountKey = transfer.lookupTable;
      let transaction = new web3_js_1.VersionedTransaction(compiledTx);
      let retries = 3;
      let res;
      while (retries > 0) {
        transaction.sign([transfer.payer]);
        recentBlockhash =
          (yield transfer.provider.connection.getRecentBlockhash("confirmed"))
            .blockhash;
        transaction.message.recentBlockhash = recentBlockhash;
        let serializedTx = transaction.serialize();
        try {
          console.log("serializedTx: ");
          res = yield (0, web3_js_1.sendAndConfirmRawTransaction)(
            transfer.provider.connection,
            serializedTx,
            {
              commitment: "confirmed",
              preflightCommitment: "confirmed",
            },
          );
          retries = 0;
        } catch (e) {
          console.log(e);
          retries--;
          if (retries == 0 || e.logs != undefined) {
            const ixClose = yield transfer.verifier.verifierProgram.methods
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
    });
  }
  sendTransaction(insert = true) {
    return __awaiter(this, void 0, void 0, function* () {
      (0, chai_1.assert)(this.nullifierPdaPubkeys.length == 10);
      let balance = yield this.provider.connection.getBalance(
        this.signerAuthorityPubkey,
        { preflightCommitment: "confirmed", commitment: "confirmed" },
      );
      if (balance === 0) {
        yield this.provider.connection.confirmTransaction(
          yield this.provider.connection.requestAirdrop(
            this.signerAuthorityPubkey,
            1000000000,
          ),
          { preflightCommitment: "confirmed", commitment: "confirmed" },
        );
      }
      try {
        this.recipientBalancePriorTx = (yield (0, spl_token_1.getAccount)(
          this.provider.connection,
          this.recipient,
          spl_token_1.TOKEN_PROGRAM_ID,
        )).amount;
      } catch (error) {}
      this.recipientFeeBalancePriorTx =
        yield this.provider.connection.getBalance(this.recipientFee);
      // console.log("recipientBalancePriorTx: ", this.recipientBalancePriorTx);
      // console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);
      // console.log("sender_fee: ", this.senderFee);
      this.senderFeeBalancePriorTx = yield this.provider.connection.getBalance(
        this.senderFee,
      );
      this.relayerRecipientAccountBalancePriorLastTx =
        yield this.provider.connection.getBalance(this.relayerRecipient);
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
      console.log(
        "this.verifierStatePubkey, ",
        this.verifierStatePubkey.toBase58(),
      );
      // console.log("this.publicInputs.nullifiers, ", this.publicInputs.nullifiers);
      // console.log("this.rootIndex ", this.rootIndex);
      // console.log("this.relayerFee ", this.relayerFee);
      // console.log("this.encryptedUtxos ", this.encryptedUtxos);
      // this.transferFirst = transferFirst;
      // this.transferSecond = transferSecond;
      //TODO: think about how to do this in a better way this is quite confusing since the this in this fn is not shieldedTransfer not the verifier object
      let res = yield this.verifier.transferFirst(this);
      res = yield this.verifier.transferSecond(this);
      return res;
    });
  }
}
exports.VerifierOne = VerifierOne;
