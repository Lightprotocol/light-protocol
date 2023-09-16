"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const src_1 = require("../src");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const verifiers = [
    { verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO, isApp: false },
    { verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ONE, isApp: false },
    { verifierIdl: src_1.IDL_VERIFIER_PROGRAM_TWO, isApp: true },
];
describe("Verifier tests", () => {
    before(async () => {
        await buildPoseidonOpt();
    });
    (0, mocha_1.it)("Test functional circuit", async () => {
        for (let verifier in verifiers) {
            await (0, src_1.functionalCircuitTest)(verifiers[verifier].isApp, verifiers[verifier].verifierIdl);
        }
    });
});
//# sourceMappingURL=verifiers.test.js.map