import { ProviderErrorCode, TransactionError, TransactionErrorCode, } from "../errors";
import { Provider, IDL_MERKLE_TREE_PROGRAM, checkRentExemption, Utxo, FIELD_SIZE, Action, merkleTreeProgramId, fetchRecentTransactions, MerkleTreeConfig, } from "../index";
import { BN, Program } from "@coral-xyz/anchor";
import { getAccount } from "@solana/spl-token";
var assert = require("assert");
export class TestTransaction {
    constructor({ txParams, provider, appParams, }) {
        this.merkleTreeProgram = new Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId, provider.provider);
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
            throw new TransactionError(ProviderErrorCode.PROVIDER_UNDEFINED, "getTestValues", "");
        if (!this.provider.provider)
            throw new TransactionError(ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED, "getTestValues", "Provider.provider undefined");
        if (!this.params)
            throw new TransactionError(TransactionErrorCode.TX_PARAMETERS_UNDEFINED, "getTestValues", "");
        if (!this.params.relayer)
            throw new TransactionError(TransactionErrorCode.RELAYER_UNDEFINED, "getTestValues", "");
        if (!this.params.accounts.recipientSol)
            throw new TransactionError(TransactionErrorCode.SOL_RECIPIENT_UNDEFINED, "getTestValues", "");
        if (!this.params.accounts.senderSol)
            throw new TransactionError(TransactionErrorCode.SOL_SENDER_UNDEFINED, "getTestValues", "");
        if (!this.testValues)
            throw new TransactionError(TransactionErrorCode.TRANSACTION_INPUTS_UNDEFINED, "getTestValues", "");
        if (this.params.accounts.recipientSpl) {
            try {
                this.testValues.recipientBalancePriorTx = new BN((await getAccount(this.provider.provider.connection, this.params.accounts.recipientSpl)).amount.toString());
            }
            catch (e) {
                // covers the case of the recipient being a native sol address not a spl token address
                try {
                    this.testValues.recipientBalancePriorTx = new BN(await this.provider.provider.connection.getBalance(this.params.accounts.recipientSpl));
                }
                catch (e) { }
            }
        }
        try {
            this.testValues.recipientFeeBalancePriorTx = new BN(await this.provider.provider.connection.getBalance(this.params.accounts.recipientSol));
        }
        catch (error) {
            throw error;
        }
        if (this.params.action === "SHIELD") {
            this.testValues.senderFeeBalancePriorTx = new BN(await this.provider.provider.connection.getBalance(this.params.relayer.accounts.relayerPubkey));
        }
        else {
            this.testValues.senderFeeBalancePriorTx = new BN(await this.provider.provider.connection.getBalance(this.params.accounts.senderSol));
        }
        this.testValues.relayerRecipientAccountBalancePriorLastTx = new BN(await this.provider.provider.connection.getBalance(this.params.relayer.accounts.relayerRecipientSol));
    }
    async checkBalances(transactionInputs, remainingAccounts, proofInput, account) {
        var _a, _b, _c, _d, _e, _f;
        if (!this.params)
            throw new TransactionError(TransactionErrorCode.TX_PARAMETERS_UNDEFINED, "getPdaAddresses", "");
        if (!transactionInputs.publicInputs)
            throw new TransactionError(TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED, "getPdaAddresses", "");
        if (!this.params.accounts.senderSol) {
            throw new Error("params.accounts.senderSol undefined");
        }
        if (!this.params.accounts.recipientSol) {
            throw new Error("params.accounts.recipientSol undefined");
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
        if (!this.testValues.relayerRecipientAccountBalancePriorLastTx) {
            throw new Error("test values relayerRecipientAccountBalancePriorLastTx undefined");
        }
        if (new BN(proofInput.publicAmountSpl).toString() === "0") {
            this.testValues.is_token = false;
        }
        else {
            this.testValues.is_token = true;
        }
        if (this.testValues.is_token && !this.params.accounts.senderSpl) {
            throw new Error("params.accounts.senderSpl undefined");
        }
        if (this.testValues.is_token && !this.params.accounts.recipientSpl) {
            throw new Error("params.accounts.recipientSpl undefined");
        }
        if (this.testValues.is_token && !this.testValues.recipientBalancePriorTx) {
            throw new Error("test values recipientBalancePriorTx undefined");
        }
        // Checking that nullifiers were inserted
        for (var i = 0; i < ((_a = remainingAccounts.nullifierPdaPubkeys) === null || _a === void 0 ? void 0 : _a.length); i++) {
            var nullifierAccount = await this.provider.provider.connection.getAccountInfo(remainingAccounts.nullifierPdaPubkeys[i].pubkey, {
                commitment: "processed",
            });
            await checkRentExemption({
                account: nullifierAccount,
                connection: this.provider.provider.connection,
            });
        }
        var leavesAccountData;
        // Checking that leaves were inserted
        for (var i = 0; i < remainingAccounts.leavesPdaPubkeys.length; i += 2) {
            leavesAccountData =
                await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(remainingAccounts.leavesPdaPubkeys[i / 2].pubkey, "processed");
            assert.equal(leavesAccountData.nodeLeft.toString(), transactionInputs.publicInputs.outputCommitment[i].reverse().toString(), "left leaf not inserted correctly");
            assert.equal(leavesAccountData.nodeRight.toString(), transactionInputs.publicInputs.outputCommitment[i + 1]
                .reverse()
                .toString(), "right leaf not inserted correctly");
            assert.equal(leavesAccountData.merkleTreePubkey.toBase58(), this.provider.solMerkleTree.pubkey.toBase58(), "merkleTreePubkey not inserted correctly");
            let lightProvider = await Provider.loadMock();
            for (var j = 0; j < this.params.encryptedUtxos.length / 256; j++) {
                let decryptedUtxo1 = await Utxo.decrypt({
                    poseidon: this.provider.poseidon,
                    encBytes: this.params.encryptedUtxos,
                    account: account ? account : this.params.outputUtxos[0].account,
                    index: 0,
                    merkleTreePdaPublicKey: this.params.accounts.transactionMerkleTree,
                    commitment: j === 0
                        ? Buffer.from(leavesAccountData.nodeLeft)
                        : Buffer.from(leavesAccountData.nodeRight),
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                });
                if (decryptedUtxo1 !== null) {
                    Utxo.equal(this.provider.poseidon, decryptedUtxo1, this.params.outputUtxos[0], true);
                }
            }
        }
        try {
            const merkleTreeAfterUpdate = await this.merkleTreeProgram.account.transactionMerkleTree.fetch(MerkleTreeConfig.getTransactionMerkleTreePda(), "confirmed");
            leavesAccountData =
                await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(remainingAccounts.leavesPdaPubkeys[0].pubkey, "confirmed");
            assert.equal(Number(merkleTreeAfterUpdate.nextQueuedIndex), Number(leavesAccountData.leftLeafIndex) +
                remainingAccounts.leavesPdaPubkeys.length * 2);
        }
        catch (e) {
            console.log("preInsertedLeavesIndex: ", e);
            throw e;
        }
        var nrInstructions;
        if (this.appParams) {
            nrInstructions = 2;
        }
        else if (this.params) {
            nrInstructions = this.params.inputUtxos.length === 2 ? 1 : 2;
            if (this.params.message) {
                nrInstructions = Math.ceil(this.params.message.length / 900) + 1;
            }
        }
        else {
            throw new Error("No params provided.");
        }
        if (this.params.action == "SHIELD" && this.testValues.is_token == false) {
            var recipientSolAccountBalance = await this.provider.provider.connection.getBalance(this.params.accounts.recipientSol, "confirmed");
            var senderFeeAccountBalance = await this.provider.provider.connection.getBalance(this.params.relayer.accounts.relayerPubkey, "confirmed");
            assert.equal(recipientSolAccountBalance, Number(this.testValues.recipientFeeBalancePriorTx) +
                Number(this.params.publicAmountSol));
            assert.equal(new BN(this.testValues.senderFeeBalancePriorTx)
                .sub(this.params.publicAmountSol)
                .sub(new BN(5000 * nrInstructions))
                .toString(), senderFeeAccountBalance.toString());
        }
        else if (this.params.action == "SHIELD" &&
            this.testValues.is_token == true) {
            var recipientAccount = await getAccount(this.provider.provider.connection, this.params.accounts.recipientSpl);
            var recipientSolAccountBalance = await this.provider.provider.connection.getBalance(this.params.accounts.recipientSol);
            assert.equal(recipientAccount.amount.toString(), (Number(this.testValues.recipientBalancePriorTx) +
                Number(this.params.publicAmountSpl)).toString(), "amount not transferred correctly");
            if (!this.params.accounts.signingAddress)
                throw new Error("Signing address undefined");
            var senderFeeAccountBalance = await this.provider.provider.connection.getBalance(this.params.accounts.signingAddress, "confirmed");
            assert.equal(recipientSolAccountBalance, Number(this.testValues.recipientFeeBalancePriorTx) +
                Number(this.params.publicAmountSol));
            assert.equal(new BN(this.testValues.senderFeeBalancePriorTx)
                .sub(this.params.publicAmountSol)
                .sub(new BN(5000 * nrInstructions))
                .toString(), senderFeeAccountBalance.toString());
        }
        else if (this.params.action == "UNSHIELD" &&
            this.testValues.is_token == false) {
            var relayerAccount = await this.provider.provider.connection.getBalance(this.params.relayer.accounts.relayerRecipientSol, "confirmed");
            var recipientFeeAccount = await this.provider.provider.connection.getBalance(this.params.accounts.recipientSol, "confirmed");
            assert.equal(new BN(recipientFeeAccount)
                .add(new BN(this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString()))
                .toString(), new BN(this.testValues.recipientFeeBalancePriorTx)
                .sub((_b = this.params.publicAmountSol) === null || _b === void 0 ? void 0 : _b.sub(FIELD_SIZE).mod(FIELD_SIZE))
                .toString());
            assert.equal(new BN(relayerAccount)
                .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
                .toString(), (_c = this.testValues.relayerRecipientAccountBalancePriorLastTx) === null || _c === void 0 ? void 0 : _c.toString());
        }
        else if (this.params.action == "UNSHIELD" &&
            this.testValues.is_token == true) {
            await getAccount(this.provider.provider.connection, this.params.accounts.senderSpl);
            var recipientAccount = await getAccount(this.provider.provider.connection, this.params.accounts.recipientSpl);
            assert.equal(recipientAccount.amount.toString(), new BN(this.testValues.recipientBalancePriorTx)
                .sub((_d = this.params.publicAmountSpl) === null || _d === void 0 ? void 0 : _d.sub(FIELD_SIZE).mod(FIELD_SIZE))
                .toString(), "amount not transferred correctly");
            var relayerAccount = await this.provider.provider.connection.getBalance(this.params.relayer.accounts.relayerRecipientSol, "confirmed");
            var recipientFeeAccount = await this.provider.provider.connection.getBalance(this.params.accounts.recipientSol, "confirmed");
            assert.equal(new BN(recipientFeeAccount)
                .add(new BN(this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString()))
                .toString(), new BN(this.testValues.recipientFeeBalancePriorTx)
                .sub((_e = this.params.publicAmountSol) === null || _e === void 0 ? void 0 : _e.sub(FIELD_SIZE).mod(FIELD_SIZE))
                .toString());
            assert.equal(new BN(relayerAccount)
                .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
                // .add(new BN("5000"))
                .toString(), (_f = this.testValues.relayerRecipientAccountBalancePriorLastTx) === null || _f === void 0 ? void 0 : _f.toString());
        }
        else if (this.params.action === Action.TRANSFER) {
            console.log("balance check for transfer not implemented");
        }
        else {
            throw Error("mode not supplied");
        }
        if (this.params.message) {
            const indexedTransactions = await fetchRecentTransactions({
                connection: this.provider.provider.connection,
                batchOptions: {
                    limit: 5000,
                },
            });
            indexedTransactions.sort((a, b) => b.blockTime - a.blockTime);
            assert.equal(indexedTransactions[0].message.toString(), this.params.message.toString());
        }
    }
}
//# sourceMappingURL=testTransaction.js.map