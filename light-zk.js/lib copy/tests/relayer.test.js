"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const chai_1 = require("chai");
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const src_1 = require("../src");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let mockKeypair = web3_js_1.Keypair.generate();
let mockKeypair1 = web3_js_1.Keypair.generate();
let relayerFee = new anchor_1.BN("123214");
let relayerRecipientSol = web3_js_1.Keypair.generate().publicKey;
describe("Test Relayer Functional", () => {
    (0, mocha_1.it)("Relayer Deposit", () => {
        let relayer = new src_1.Relayer(mockKeypair.publicKey, mockKeypair1.publicKey, src_1.BN_1);
        chai_1.assert.equal(relayer.accounts.relayerRecipientSol.toBase58(), mockKeypair1.publicKey.toBase58());
        chai_1.assert.equal(relayer.accounts.relayerPubkey.toBase58(), mockKeypair.publicKey.toBase58());
        chai_1.assert.equal(relayer.relayerFee.toString(), "1");
    });
    (0, mocha_1.it)("Relayer Transfer/Withdrawal", () => {
        let relayer = new src_1.Relayer(mockKeypair.publicKey, relayerRecipientSol, relayerFee);
        chai_1.assert.equal(relayer.accounts.relayerPubkey.toBase58(), mockKeypair.publicKey.toBase58());
        chai_1.assert.equal(relayer.relayerFee.toString(), relayerFee.toString());
        chai_1.assert.equal(relayer.accounts.relayerRecipientSol.toBase58(), relayerRecipientSol.toBase58());
    });
    (0, mocha_1.it)("Relayer ataCreationFee", () => {
        let relayer = new src_1.Relayer(mockKeypair.publicKey);
        chai_1.assert.equal(relayer.relayerFee.toString(), "0");
        chai_1.assert.equal(src_1.TOKEN_ACCOUNT_FEE.toNumber(), relayer.getRelayerFee(true).toNumber());
        chai_1.assert.equal(src_1.BN_0.toNumber(), relayer.getRelayerFee(false).toNumber());
    });
});
describe("Test Relayer Errors", () => {
    (0, mocha_1.it)("RELAYER_PUBKEY_UNDEFINED", () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            new src_1.Relayer();
        })
            .to.throw(src_1.RelayerError)
            .includes({
            code: src_1.RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("RELAYER_FEE_UNDEFINED", () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            new src_1.Relayer(mockKeypair.publicKey, relayerRecipientSol);
        })
            .to.throw(src_1.RelayerError)
            .includes({
            code: src_1.RelayerErrorCode.RELAYER_FEE_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("RELAYER_RECIPIENT_UNDEFINED", () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            new src_1.Relayer(mockKeypair.publicKey, undefined, relayerFee);
        })
            .to.throw(src_1.RelayerError)
            .includes({
            code: src_1.RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED,
            functionName: "constructor",
        });
    });
});
//# sourceMappingURL=relayer.test.js.map