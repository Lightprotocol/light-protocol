"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const chai_1 = require("chai");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const src_1 = require("../src");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("convertAndComputeDecimals", () => {
    mocha_1.it.skip("random test", () => {
        const getRandomElement = () => {
            return parseInt(Math.floor(Math.random() * 7).toString());
        };
        for (let i = 0; i < 100000; i++) {
            let decimalsNumber = new anchor_1.BN(getRandomElement());
            console.log("decimals ", decimalsNumber);
            let decimals = new anchor_1.BN(10).pow(new anchor_1.BN(decimalsNumber));
            const amount = (0, src_1.generateRandomTestAmount)(0, 1000000000, decimalsNumber.toNumber());
            console.log("amount ", amount);
            console.log("decimals ", decimals.toString());
            const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
            (0, chai_1.expect)(result.toString()).to.equal(Math.round(amount * decimals.toNumber()).toString());
        }
    });
    (0, mocha_1.it)("should correctly convert number (integer) values", () => {
        const amount = 3;
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("30");
    });
    (0, mocha_1.it)("should correctly convert number (float) values", () => {
        const amount = "2.5";
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("25");
    });
    (0, mocha_1.it)("should correctly convert number (float) values", () => {
        const amount = 54.08;
        const decimals = new anchor_1.BN(100);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("5408");
    });
    (0, mocha_1.it)("should correctly convert string values", () => {
        const amount = "4";
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("40");
    });
    (0, mocha_1.it)("should correctly convert BN values", () => {
        const amount = new anchor_1.BN(5);
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("50");
    });
    (0, mocha_1.it)("should correctly handle zero amount", () => {
        const amount = src_1.BN_0;
        const decimals = new anchor_1.BN(100);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("0");
    });
    (0, mocha_1.it)("should handle zero decimals correctly", () => {
        const amount = "5";
        const decimals = src_1.BN_1;
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("5");
    });
    (0, mocha_1.it)("should throw an error for negative decimals", () => {
        const amount = 5;
        const decimals = new anchor_1.BN(-10);
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw();
    });
    (0, mocha_1.it)("should correctly handle very large amount", () => {
        const amount = 1e18; // One quintillion
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("10000000000000000000");
    });
    (0, mocha_1.it)("should correctly handle max u64 amount", () => {
        const amount = new anchor_1.BN("18446744073709551615"); // max u64
        const decimals = src_1.BN_1;
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("18446744073709551615");
    });
    (0, mocha_1.it)("should throw because the output is still a float", () => {
        const amount = 0.01; // One hundred-thousandth
        const decimals = new anchor_1.BN(10);
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw();
    });
    (0, mocha_1.it)("should correctly handle string with decimal points", () => {
        const amount = "2.5";
        const decimals = new anchor_1.BN(10);
        const result = (0, src_1.convertAndComputeDecimals)(amount, decimals);
        (0, chai_1.expect)(result.toString()).to.equal("25");
    });
    (0, mocha_1.it)("should throw an error for invalid string", () => {
        const amount = "invalid";
        const decimals = new anchor_1.BN(10);
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw();
    });
    (0, mocha_1.it)("should throw an error for null amount", () => {
        const amount = null;
        const decimals = new anchor_1.BN(10);
        // @ts-ignore: ignore for testing
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw();
    });
    (0, mocha_1.it)("should throw an error for undefined amount", () => {
        const amount = undefined;
        const decimals = new anchor_1.BN(10);
        // @ts-ignore: ignore for testing
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw();
    });
    (0, mocha_1.it)("should throw an error for negative amount", () => {
        const amount = -3;
        const decimals = new anchor_1.BN(10);
        (0, chai_1.expect)(() => (0, src_1.convertAndComputeDecimals)(amount, decimals)).to.throw("Negative amounts are not allowed.");
    });
});
//# sourceMappingURL=convertDecimals.test.js.map