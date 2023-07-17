"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.performMergeUtxos = exports.performMergeAll = exports.performShielding = void 0;
const zk_js_1 = require("@lightprotocol/zk.js");
function performShielding({ numberOfShields = 1, testInputs, environmentConfig, }) {
    return __awaiter(this, void 0, void 0, function* () {
        if (!testInputs.recipientSeed && testInputs.shieldToRecipient)
            throw new Error("testinputs recipientSeed is undefined");
        for (var i = 0; i < numberOfShields; i++) {
            const provider = yield zk_js_1.Provider.init({
                wallet: environmentConfig.providerSolanaKeypair,
                relayer: environmentConfig.relayer,
            });
            const userSender = yield zk_js_1.User.init({
                provider,
            });
            const userRecipient = testInputs.shieldToRecipient
                ? yield zk_js_1.User.init({
                    provider,
                    seed: testInputs.recipientSeed,
                })
                : userSender;
            const testStateValidator = new zk_js_1.UserTestAssertHelper({
                userSender,
                userRecipient,
                provider,
                testInputs,
            });
            yield testStateValidator.fetchAndSaveState();
            if (testInputs.shieldToRecipient) {
                yield userSender.shield({
                    publicAmountSol: testInputs.amountSol,
                    publicAmountSpl: testInputs.amountSpl,
                    token: testInputs.token,
                    recipient: userRecipient.account.getPublicKey(),
                });
            }
            else {
                yield userSender.shield({
                    publicAmountSol: testInputs.amountSol,
                    publicAmountSpl: testInputs.amountSpl,
                    token: testInputs.token,
                });
            }
            yield userRecipient.provider.latestMerkleTree();
            if (testInputs.token === "SOL" && testInputs.type === zk_js_1.Action.SHIELD) {
                // await testStateValidator.checkSolShielded();
            }
            else if (testInputs.token !== "SOL" &&
                testInputs.type === zk_js_1.Action.SHIELD) {
                yield testStateValidator.checkSplShielded();
            }
            else {
                throw new Error(`No test option found for testInputs ${testInputs}`);
            }
            testInputs.expectedUtxoHistoryLength++;
        }
    });
}
exports.performShielding = performShielding;
function performMergeAll({ testInputs, environmentConfig, }) {
    return __awaiter(this, void 0, void 0, function* () {
        if (!testInputs.recipientSeed)
            throw new Error("testinputs recipientSeed is undefined");
        const provider = yield zk_js_1.Provider.init({
            wallet: environmentConfig.providerSolanaKeypair,
            relayer: environmentConfig.relayer,
        });
        const userSender = yield zk_js_1.User.init({
            provider,
            seed: testInputs.recipientSeed,
        });
        yield userSender.getUtxoInbox();
        const testStateValidator = new zk_js_1.UserTestAssertHelper({
            userSender,
            userRecipient: userSender,
            provider,
            testInputs,
        });
        yield testStateValidator.fetchAndSaveState();
        yield userSender.mergeAllUtxos(testStateValidator.tokenCtx.mint);
        /**
         * Test:
         * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
         * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
         * -
         */
        // TODO: add random amount and amount checks
        yield userSender.provider.latestMerkleTree();
        yield testStateValidator.checkMergedAll();
    });
}
exports.performMergeAll = performMergeAll;
function performMergeUtxos({ testInputs, environmentConfig, }) {
    return __awaiter(this, void 0, void 0, function* () {
        if (!testInputs.recipientSeed)
            throw new Error("testinputs recipientSeed is undefined");
        const provider = yield zk_js_1.Provider.init({
            wallet: environmentConfig.providerSolanaKeypair,
            relayer: environmentConfig.relayer,
        });
        const userSender = yield zk_js_1.User.init({
            provider,
            seed: testInputs.recipientSeed,
        });
        yield userSender.getUtxoInbox();
        const testStateValidator = new zk_js_1.UserTestAssertHelper({
            userSender,
            userRecipient: userSender,
            provider,
            testInputs,
        });
        yield testStateValidator.fetchAndSaveState();
        yield userSender.mergeUtxos(testInputs.utxoCommitments, testStateValidator.tokenCtx.mint);
        /**
         * Test:
         * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
         * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
         * -
         */
        // TODO: add random amount and amount checks
        yield userSender.provider.latestMerkleTree();
        yield testStateValidator.checkMerged();
    });
}
exports.performMergeUtxos = performMergeUtxos;
