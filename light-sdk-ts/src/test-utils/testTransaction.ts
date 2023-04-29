import {
  ProviderErrorCode,
  TransactionError,
  TransactionErrorCode,
  TransactioParametersError,
} from "../errors";
import {
  Account,
  TransactionParameters,
  Provider,
  IDL_MERKLE_TREE_PROGRAM,
  checkRentExemption,
  Utxo,
  MERKLE_TREE_KEY,
  FIELD_SIZE,
  Action,
  merkleTreeProgramId,
} from "../index";
import { BN, Program } from "@coral-xyz/anchor";
import { getAccount } from "@solana/spl-token";
var assert = require("assert");

export class TestTransaction {
  testValues?: {
    recipientBalancePriorTx?: BN;
    relayerRecipientAccountBalancePriorLastTx?: BN;
    txIntegrityHash?: BN;
    senderFeeBalancePriorTx?: BN;
    recipientFeeBalancePriorTx?: BN;
    is_token?: boolean;
  };
  params: TransactionParameters;
  provider: Provider;
  merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
  appParams?: any;

  constructor({
    txParams,
    provider,
    appParams,
  }: {
    txParams: TransactionParameters;
    appParams?: any;
    provider: Provider;
  }) {
    this.merkleTreeProgram = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider.provider,
    );
    this.params = txParams;
    this.provider = provider;
    this.appParams = appParams;
    this.testValues = {};
  }

  // send transaction should be the same for both deposit and withdrawal
  // the function should just send the tx to the rpc or relayer respectively
  // in case there is more than one transaction to be sent to the verifier these can be sent separately
  // TODO: make optional and default no
  async getTestValues() {
    if (!this.provider)
      throw new TransactionError(
        ProviderErrorCode.PROVIDER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.provider.provider)
      throw new TransactionError(
        ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
        "getTestValues",
        "Provider.provider undefined",
      );
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.recipientSpl)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.recipientSol)
      throw new TransactionError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.senderSol)
      throw new TransactionError(
        TransactionErrorCode.SOL_SENDER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.testValues)
      throw new TransactionError(
        TransactionErrorCode.TRANSACTION_INPUTS_UNDEFINED,
        "getTestValues",
        "",
      );

    try {
      this.testValues.recipientBalancePriorTx = new BN(
        (
          await getAccount(
            this.provider.provider.connection,
            this.params.accounts.recipientSpl,
          )
        ).amount.toString(),
      );
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        this.testValues.recipientBalancePriorTx = new BN(
          await this.provider.provider.connection.getBalance(
            this.params.accounts.recipientSpl,
          ),
        );
      } catch (e) {}
    }

    try {
      this.testValues.recipientFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientSol,
        ),
      );
    } catch (error) {
      console.log(
        "this.testValues.recipientFeeBalancePriorTx fetch failed ",
        this.params.accounts.recipientSol,
      );
    }
    if (this.params.action === "SHIELD") {
      this.testValues.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
        ),
      );
    } else {
      this.testValues.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.senderSol,
        ),
      );
    }

    this.testValues.relayerRecipientAccountBalancePriorLastTx = new BN(
      await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipientSol,
      ),
    );
  }

  async checkBalances(
    transactionInputs: any,
    remainingAccounts: any,
    proofInput: any,
    account?: Account,
  ) {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getPdaAddresses",
        "",
      );

    if (!this.params.accounts.senderSol) {
      throw new Error("params.accounts.senderSol undefined");
    }

    if (!this.params.accounts.recipientSol) {
      throw new Error("params.accounts.recipientSol undefined");
    }

    if (!this.params.accounts.recipientSpl) {
      throw new Error("params.accounts.recipientSpl undefined");
    }

    if (!this.params.accounts.recipientSpl) {
      throw new Error("params.accounts.recipientSpl undefined");
    }
    if (!this.testValues) {
      throw new Error("test values undefined");
    }
    if (!this.testValues.senderFeeBalancePriorTx) {
      throw new Error("senderFeeBalancePriorTx undefined");
    }

    if (!this.params.publicAmountSol) {
      throw new Error("amountSol undefined");
    }

    if (!this.params.publicAmountSol) {
      throw new Error("amountSol undefined");
    }

    if (!this.merkleTreeProgram) {
      throw new Error("merkleTreeProgram undefined");
    }
    this.provider.solMerkleTree;

    if (!this.provider) {
      throw new Error("provider undefined");
    }

    if (!this.provider.solMerkleTree) {
      throw new Error("provider.solMerkleTree undefined");
    }

    if (!this.params.encryptedUtxos) {
      throw new Error("params.encryptedUtxos undefined");
    }

    if (!this.params.outputUtxos) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.provider.provider) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.params.relayer) {
      throw new Error("params.relayer undefined");
    }

    if (!this.params.accounts.senderSpl) {
      throw new Error("params.accounts.senderSpl undefined");
    }
    if (!remainingAccounts) {
      throw new Error("remainingAccounts.nullifierPdaPubkeys undefined");
    }
    if (!remainingAccounts.nullifierPdaPubkeys) {
      throw new Error("remainingAccounts.nullifierPdaPubkeys undefined");
    }

    if (!remainingAccounts.leavesPdaPubkeys) {
      throw new Error("remainingAccounts.leavesPdaPubkeys undefined");
    }
    if (!this.testValues) {
      throw new Error("test values undefined");
    }
    if (!this.testValues.recipientFeeBalancePriorTx) {
      throw new Error("test values recipientFeeBalancePriorTx undefined");
    }

    if (!this.testValues.recipientBalancePriorTx) {
      throw new Error("test values recipientBalancePriorTx undefined");
    }

    if (!this.testValues.relayerRecipientAccountBalancePriorLastTx) {
      throw new Error(
        "test values relayerRecipientAccountBalancePriorLastTx undefined",
      );
    }
    // Checking that nullifiers were inserted
    if (new BN(proofInput.publicAmountSpl).toString() === "0") {
      this.testValues.is_token = false;
    } else {
      this.testValues.is_token = true;
    }
    for (var i = 0; i < remainingAccounts.nullifierPdaPubkeys?.length; i++) {
      var nullifierAccount =
        await this.provider.provider!.connection.getAccountInfo(
          remainingAccounts.nullifierPdaPubkeys[i].pubkey,
          {
            commitment: "confirmed",
          },
        );

      await checkRentExemption({
        account: nullifierAccount,
        connection: this.provider.provider!.connection,
      });
    }
    let leavesAccount;
    var leavesAccountData;
    // Checking that leaves were inserted
    for (var i = 0; i < remainingAccounts.leavesPdaPubkeys.length; i++) {
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          remainingAccounts.leavesPdaPubkeys[i].pubkey,
          "confirmed",
        );

      assert(
        leavesAccountData.nodeLeft.toString() ==
          transactionInputs.publicInputs.leaves[i][0].reverse().toString(),
        "left leaf not inserted correctly",
      );
      assert(
        leavesAccountData.nodeRight.toString() ==
          transactionInputs.publicInputs.leaves[i][1].reverse().toString(),
        "right leaf not inserted correctly",
      );
      assert(
        leavesAccountData.merkleTreePubkey.toBase58() ==
          this.provider.solMerkleTree.pubkey.toBase58(),
        "merkleTreePubkey not inserted correctly",
      );
      for (var j = 0; j < this.params.encryptedUtxos.length / 256; j++) {
        // console.log(j);

        if (
          leavesAccountData.encryptedUtxos.toString() !==
          this.params.encryptedUtxos.toString()
        ) {
          // console.log(j);
          // throw `encrypted utxo ${i} was not stored correctly`;
        }
        // console.log(
        //   `${leavesAccountData.encryptedUtxos} !== ${this.params.encryptedUtxos}`
        // );

        // assert(leavesAccountData.encryptedUtxos === this.encryptedUtxos, "encryptedUtxos not inserted correctly");
        // TODO: add for both utxos of leafpda
        let decryptedUtxo1 = await Utxo.decrypt({
          poseidon: this.provider.poseidon,
          encBytes: this.params!.encryptedUtxos!,
          account: account ? account : this.params!.outputUtxos![0].account,
          index: 0, // this is just a placeholder
          transactionIndex: this.params!.transactionIndex,
          merkleTreePdaPublicKey: this.params!.accounts.transactionMerkleTree,
          commitment: new BN(
            this.params!.outputUtxos![0].getCommitment(this.provider.poseidon),
          ).toBuffer(),
        });
        const utxoEqual = (utxo0: Utxo, utxo1: Utxo) => {
          assert.equal(
            utxo0.amounts[0].toString(),
            utxo1.amounts[0].toString(),
          );
          assert.equal(
            utxo0.amounts[1].toString(),
            utxo1.amounts[1].toString(),
          );
          assert.equal(utxo0.assets[0].toString(), utxo1.assets[0].toString());
          assert.equal(utxo0.assets[1].toString(), utxo1.assets[1].toString());
          assert.equal(
            utxo0.assetsCircuit[0].toString(),
            utxo1.assetsCircuit[0].toString(),
          );
          assert.equal(
            utxo0.assetsCircuit[1].toString(),
            utxo1.assetsCircuit[1].toString(),
          );
          assert.equal(
            utxo0.appDataHash.toString(),
            utxo1.appDataHash.toString(),
          );
          assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
          assert.equal(
            utxo0.verifierAddress.toString(),
            utxo1.verifierAddress.toString(),
          );
          assert.equal(
            utxo0.verifierAddressCircuit.toString(),
            utxo1.verifierAddressCircuit.toString(),
          );
        };
        // console.log("decryptedUtxo ", decryptedUtxo1);
        // console.log("this.params.outputUtxos[0] ", this.params.outputUtxos[0]);
        if (decryptedUtxo1 !== null) {
          utxoEqual(decryptedUtxo1, this.params.outputUtxos[0]);
        } else {
          console.log("Could not decrypt any utxo probably a withdrawal.");
        }
      }
    }

    console.log(
      `mode ${this.params.action}, this.testValues.is_token ${this.testValues.is_token}`,
    );

    try {
      const merkleTreeAfterUpdate =
        await this.merkleTreeProgram.account.transactionMerkleTree.fetch(
          MERKLE_TREE_KEY,
          "confirmed",
        );
      console.log(
        "Number(merkleTreeAfterUpdate.nextQueuedIndex) ",
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
      );
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          remainingAccounts.leavesPdaPubkeys[0].pubkey,
          "confirmed",
        );
      console.log(
        `${Number(leavesAccountData.leftLeafIndex)} + ${
          remainingAccounts.leavesPdaPubkeys.length * 2
        }`,
      );

      assert.equal(
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
        Number(leavesAccountData.leftLeafIndex) +
          remainingAccounts.leavesPdaPubkeys.length * 2,
      );
    } catch (e) {
      console.log("preInsertedLeavesIndex: ", e);
    }
    var nrInstructions;
    if (this.appParams) {
      nrInstructions = this.appParams.verifier.instructions?.length;
    } else if (this.params) {
      nrInstructions = this.params.verifier.instructions?.length;
    } else {
      throw new Error("No params provided.");
    }
    console.log("nrInstructions ", nrInstructions);

    if (this.params.action == "SHIELD" && this.testValues.is_token == false) {
      var recipientSolAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientSol,
          "confirmed",
        );
      console.log(
        "testValues.recipientFeeBalancePriorTx: ",
        this.testValues.recipientFeeBalancePriorTx,
      );

      var senderFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
          "confirmed",
        );
      assert(
        recipientSolAccountBalance ==
          Number(this.testValues.recipientFeeBalancePriorTx) +
            Number(this.params.publicAmountSol),
      );

      console.log(
        `prior ${new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString()} ==  now ${senderFeeAccountBalance}, diff ${new BN(
          this.testValues.senderFeeBalancePriorTx,
        )
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .sub(new BN(senderFeeAccountBalance))}`,
      );
      assert.equal(
        new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString(),
        senderFeeAccountBalance.toString(),
      );
    } else if (
      this.params.action == "SHIELD" &&
      this.testValues.is_token == true
    ) {
      console.log("SHIELD and token");

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipientSpl,
      );
      var recipientSolAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientSol,
        );

      // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
      // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.extAmount, 0)).toString(), "amount not transferred correctly");

      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${this.testValues.recipientBalancePriorTx}`,
      );
      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${
          Number(this.testValues.recipientBalancePriorTx) +
          Number(this.params.publicAmountSpl)
        }`,
      );
      assert(
        recipientAccount.amount.toString() ===
          (
            Number(this.testValues.recipientBalancePriorTx) +
            Number(this.params.publicAmountSpl)
          ).toString(),
        "amount not transferred correctly",
      );
      console.log(
        `Blanace now ${recipientSolAccountBalance} ${
          Number(this.testValues.recipientFeeBalancePriorTx) +
          Number(this.params.publicAmountSol)
        }`,
      );
      console.log("fee amount: ", this.params.publicAmountSol);
      console.log(
        "fee amount from inputs. ",
        new BN(
          transactionInputs.publicInputs.publicAmountSol.slice(24, 32),
        ).toString(),
      );
      console.log(
        "pub amount from inputs. ",
        new BN(
          transactionInputs.publicInputs.publicAmountSpl.slice(24, 32),
        ).toString(),
      );

      var senderFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.senderSol,
          "confirmed",
        );

      assert(
        recipientSolAccountBalance ==
          Number(this.testValues.recipientFeeBalancePriorTx) +
            Number(this.params.publicAmountSol),
      );
      console.log(
        `${new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString()} == ${senderFeeAccountBalance}`,
      );
      assert(
        new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString() == senderFeeAccountBalance.toString(),
      );
    } else if (
      this.params.action == "UNSHIELD" &&
      this.testValues.is_token == false
    ) {
      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipientSol,
        "confirmed",
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientSol,
          "confirmed",
        );

      console.log(
        "testValues.relayerRecipientAccountBalancePriorLastTx ",
        this.testValues.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString()} == ${new BN(
          this.testValues.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );
      console.log(
        `recipientFeeAccount ${new BN(recipientFeeAccount)
          .add(
            new BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString()}  == ${new BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new BN(recipientFeeAccount)
          .add(
            new BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString(),
        new BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );
      console.log(
        `this.params.relayer.relayerFee ${
          this.params.relayer.relayerFee
        } new BN(relayerAccount) ${new BN(relayerAccount).toString()}`,
      );
      console.log(
        `relayerRecipientAccountBalancePriorLastTx ${this.testValues.relayerRecipientAccountBalancePriorLastTx?.toString()}`,
      );

      assert.equal(
        new BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString(),
        this.testValues.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else if (
      this.params.action == "UNSHIELD" &&
      this.testValues.is_token == true
    ) {
      var senderAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.senderSpl,
      );

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipientSpl,
      );

      // assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
      console.log(
        "this.testValues.recipientBalancePriorTx ",
        this.testValues.recipientBalancePriorTx,
      );
      console.log("this.params.publicAmountSpl ", this.params.publicAmountSpl);
      console.log(
        "this.params.publicAmountSpl ",
        this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE),
      );

      console.log(
        `${recipientAccount.amount}, ${new BN(
          this.testValues.recipientBalancePriorTx,
        )
          .sub(this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );
      assert.equal(
        recipientAccount.amount.toString(),
        new BN(this.testValues.recipientBalancePriorTx)
          .sub(this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
        "amount not transferred correctly",
      );

      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipientSol,
        "confirmed",
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientSol,
          "confirmed",
        );

      // console.log("relayerAccount ", relayerAccount);
      // console.log("this.params.relayer.relayerFee: ", this.params.relayer.getRelayerFee);
      console.log(
        "testValues.relayerRecipientAccountBalancePriorLastTx ",
        this.testValues.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString()} == ${new BN(
          this.testValues.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );

      console.log(
        `recipientFeeAccount ${new BN(recipientFeeAccount)
          .add(
            new BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString()}  == ${new BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new BN(recipientFeeAccount)
          .add(
            new BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString(),
        new BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );

      assert.equal(
        new BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          // .add(new BN("5000"))
          .toString(),
        this.testValues.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else if (this.params.action === Action.TRANSFER) {
      console.log("balance check for transfer not implemented");
    } else {
      throw Error("mode not supplied");
    }
  }
}
