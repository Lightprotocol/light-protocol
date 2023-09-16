"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
//@ts-nocheck
const chai_1 = require("chai");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const anchor_1 = require("@coral-xyz/anchor");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const web3_js_1 = require("@solana/web3.js");
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildBabyjub, buildEddsa } = circomlibjs;
const src_1 = require("../src");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
const numberMaxOutUtxos = 2;
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
describe("Test createOutUtxos Functional", () => {
    let poseidon, eddsa, babyJub, F, k0, k00, kBurner;
    let splAmount, solAmount, token, tokenCtx, utxo1, relayerFee, utxoSol, recipientAccount, lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await circomlibjs.buildPoseidonOpt();
        eddsa = await buildEddsa();
        babyJub = await buildBabyjub();
        F = babyJub.F;
        k0 = new src_1.Account({ poseidon, seed: seed32 });
        k00 = new src_1.Account({ poseidon, seed: seed32 });
        kBurner = src_1.Account.createBurner(poseidon, seed32, new anchor_1.BN("0"));
        splAmount = new anchor_1.BN(3);
        solAmount = new anchor_1.BN(1e6);
        token = "USDC";
        const tmpTokenCtx = src_1.TOKEN_REGISTRY.get(token);
        if (!tmpTokenCtx)
            throw new Error("Token not supported!");
        tokenCtx = tmpTokenCtx;
        splAmount = splAmount.mul(new anchor_1.BN(tokenCtx.decimals));
        utxo1 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e8), new anchor_1.BN(5 * tokenCtx.decimals.toNumber())],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxoSol = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            amounts: [new anchor_1.BN(1e6)],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        relayerFee = src_1.RELAYER_FEE;
        let recipientAccountRoot = new src_1.Account({
            poseidon,
            seed: bytes_1.bs58.encode(new Uint8Array(32).fill(3)),
        });
        recipientAccount = src_1.Account.fromPubkey(recipientAccountRoot.getPublicKey(), poseidon);
    });
    (0, mocha_1.it)("shield sol", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: src_1.BN_0,
            publicAmountSol: solAmount,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.SHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), solAmount.toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), "0", `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("shield spl", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: new anchor_1.BN(10),
            publicAmountSol: src_1.BN_0,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.SHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), "0", `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), "10", `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("shield sol with input utxo", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: src_1.BN_0,
            publicAmountSol: solAmount,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.SHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].add(solAmount).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("shield sol & spl with input utxo", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: new anchor_1.BN(10),
            publicAmountSol: solAmount,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.SHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].add(solAmount).toString(), `${outUtxos[0].amounts[0].add(solAmount)} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].add(new anchor_1.BN("10")).toString(), `${utxo1.amounts[1].add(new anchor_1.BN("10")).toString()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("shield sol & spl", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: new anchor_1.BN(10),
            publicAmountSol: solAmount,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.SHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), solAmount.toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), "10", `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield SPL - no relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            publicAmountSol: src_1.BN_0,
            poseidon,
            relayerFee: src_1.BN_0,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].sub(splAmount).toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield SPL - with relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            publicAmountSol: src_1.BN_0,
            poseidon,
            relayerFee,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].sub(relayerFee).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].sub(splAmount).toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield sol - no relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: src_1.BN_0,
            publicAmountSol: solAmount,
            poseidon,
            relayerFee: src_1.BN_0,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].sub(solAmount).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield sol - with relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: src_1.BN_0,
            publicAmountSol: solAmount,
            poseidon,
            relayerFee,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].sub(relayerFee).sub(solAmount).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield spl & sol - no relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            publicAmountSol: solAmount,
            poseidon,
            relayerFee: src_1.BN_0,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].sub(solAmount).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].sub(splAmount).toString(), `${outUtxos[0].amounts[1].sub(splAmount).toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield spl & sol - with relayer fee", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            inUtxos: [utxo1],
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            publicAmountSol: solAmount,
            poseidon,
            relayerFee,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toString(), utxo1.amounts[0].sub(relayerFee).sub(solAmount).toString(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0]}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].sub(splAmount).toString(), `${outUtxos[0].amounts[1].sub(splAmount).toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield in:1SOL + 1SPL should merge 2-1", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            inUtxos: [utxo1, utxoSol],
            publicAmountSol: src_1.BN_0,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toNumber(), utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0].toNumber() + utxoSol.amounts[0].toNumber()}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].sub(splAmount).toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() -
            splAmount.toNumber() * tokenCtx.decimals.toNumber()}`);
    });
    (0, mocha_1.it)("unshield in:1SPL + 1SPL should merge 2-1", async () => {
        let outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            inUtxos: [utxo1, utxo1],
            publicAmountSol: src_1.BN_0,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[0].amounts[0].toNumber(), utxo1.amounts[0].mul(src_1.BN_2).toNumber(), `${outUtxos[0].amounts[0]} fee != ${utxo1.amounts[0].toNumber() + utxo1.amounts[0].toNumber()}`);
        chai_1.assert.equal(outUtxos[0].amounts[1].toString(), utxo1.amounts[1].mul(src_1.BN_2).sub(splAmount).toString(), `${outUtxos[0].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() - splAmount.toNumber()}`);
    });
    (0, mocha_1.it)("transfer in:1 SPL ", async () => {
        let recipients = [
            {
                account: recipientAccount,
                mint: utxo1.assets[1],
                solAmount: src_1.BN_0,
                splAmount: src_1.BN_1,
            },
        ];
        let outUtxos = (0, src_1.createRecipientUtxos)({
            recipients,
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        outUtxos = (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: src_1.BN_0,
            inUtxos: [utxo1],
            outUtxos,
            relayerFee,
            publicAmountSol: src_1.BN_0,
            poseidon,
            changeUtxoAccount: k0,
            action: src_1.Action.TRANSFER,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(outUtxos[1].amounts[0].toNumber(), utxo1.amounts[0].toNumber() -
            relayerFee.toNumber() -
            outUtxos[0].amounts[0].toNumber(), `${outUtxos[1].amounts[0]} fee != ${utxo1.amounts[0].toNumber() -
            relayerFee.toNumber() -
            outUtxos[0].amounts[0].toNumber()}`);
        chai_1.assert.equal(outUtxos[1].amounts[1].toNumber(), utxo1.amounts[1].toNumber() - 1, `${outUtxos[1].amounts[1].toNumber()}  spl !=  ${utxo1.amounts[1].toNumber() - splAmount.toNumber()}`);
    });
});
// ... existing imports and code ...
describe("createRecipientUtxos", () => {
    let lightProvider;
    (0, mocha_1.it)("should create output UTXOs for each recipient", async () => {
        lightProvider = await src_1.Provider.loadMock();
        const poseidon = await circomlibjs.buildPoseidonOpt();
        const mint = src_1.MINT;
        const account1 = new src_1.Account({ poseidon, seed: seed32 });
        const account2 = new src_1.Account({
            poseidon,
            seed: new Uint8Array(32).fill(4).toString(),
        });
        const recipients = [
            {
                account: account1,
                solAmount: new anchor_1.BN(5),
                splAmount: new anchor_1.BN(10),
                mint,
            },
            {
                account: account2,
                solAmount: new anchor_1.BN(3),
                splAmount: new anchor_1.BN(7),
                mint,
            },
        ];
        const outputUtxos = (0, src_1.createRecipientUtxos)({
            recipients,
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(outputUtxos.length).to.equal(recipients.length);
        (0, chai_1.expect)(outputUtxos[0].account).to.equal(account1);
        (0, chai_1.expect)(outputUtxos[0].amounts[0].toString()).to.equal("5");
        (0, chai_1.expect)(outputUtxos[0].amounts[1].toString()).to.equal("10");
        (0, chai_1.expect)(outputUtxos[0].assets[0].equals(web3_js_1.SystemProgram.programId)).to.be.true;
        (0, chai_1.expect)(outputUtxos[0].assets[1].equals(mint)).to.be.true;
        (0, chai_1.expect)(outputUtxos[1].account).to.equal(account2);
        (0, chai_1.expect)(outputUtxos[1].amounts[0].toString()).to.equal("3");
        (0, chai_1.expect)(outputUtxos[1].amounts[1].toString()).to.equal("7");
        (0, chai_1.expect)(outputUtxos[1].assets[0].equals(web3_js_1.SystemProgram.programId)).to.be.true;
        (0, chai_1.expect)(outputUtxos[1].assets[1].equals(mint)).to.be.true;
    });
});
describe("validateUtxoAmounts", () => {
    let poseidon, assetPubkey, inUtxos, lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await circomlibjs.buildPoseidonOpt();
        assetPubkey = new web3_js_1.PublicKey(0);
        inUtxos = [
            createUtxo(poseidon, [new anchor_1.BN(5)], [assetPubkey]),
            createUtxo(poseidon, [new anchor_1.BN(3)], [assetPubkey]),
        ];
    });
    // Helper function to create a UTXO with specific amounts and assets
    function createUtxo(poseidon, amounts, assets) {
        return new src_1.Utxo({
            poseidon,
            amounts,
            assets,
            blinding: src_1.BN_0,
            account: new src_1.Account({ poseidon }),
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    }
    (0, mocha_1.it)("should not throw an error if input UTXOs sum is equal to output UTXOs sum", () => {
        const outUtxos = [createUtxo(poseidon, [new anchor_1.BN(8)], [assetPubkey])];
        (0, chai_1.expect)(() => (0, src_1.validateUtxoAmounts)({ assetPubkeys: [assetPubkey], inUtxos, outUtxos })).not.to.throw();
    });
    (0, mocha_1.it)("should not throw an error if input UTXOs sum is greater than output UTXOs sum", () => {
        const outUtxos = [createUtxo(poseidon, [new anchor_1.BN(7)], [assetPubkey])];
        (0, chai_1.expect)(() => (0, src_1.validateUtxoAmounts)({ assetPubkeys: [assetPubkey], inUtxos, outUtxos })).not.to.throw();
    });
    (0, mocha_1.it)("should throw an error if input UTXOs sum is less than output UTXOs sum", () => {
        const outUtxos = [createUtxo(poseidon, [new anchor_1.BN(9)], [assetPubkey])];
        (0, chai_1.expect)(() => (0, src_1.validateUtxoAmounts)({ assetPubkeys: [assetPubkey], inUtxos, outUtxos })).to.throw(src_1.CreateUtxoError);
    });
});
describe("Test createOutUtxos Errors", () => {
    let poseidon, eddsa, babyJub, F, k0, k00, kBurner;
    let splAmount, solAmount, token, tokenCtx, utxo1, relayerFee, utxoSol, recipientAccount, lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await circomlibjs.buildPoseidonOpt();
        eddsa = await buildEddsa();
        babyJub = await buildBabyjub();
        F = babyJub.F;
        k0 = new src_1.Account({ poseidon, seed: seed32 });
        k00 = new src_1.Account({ poseidon, seed: seed32 });
        kBurner = src_1.Account.createBurner(poseidon, seed32, new anchor_1.BN("0"));
        splAmount = new anchor_1.BN(3);
        solAmount = new anchor_1.BN(1e6);
        token = "USDC";
        let tmpTokenCtx = src_1.TOKEN_REGISTRY.get(token);
        if (!tmpTokenCtx)
            throw new Error("Token not supported!");
        tokenCtx = tmpTokenCtx;
        splAmount = splAmount.mul(new anchor_1.BN(tokenCtx.decimals));
        utxo1 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e8), new anchor_1.BN(5 * tokenCtx.decimals.toNumber())],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxoSol = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            amounts: [new anchor_1.BN(1e6)],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        relayerFee = src_1.RELAYER_FEE;
        let recipientAccountRoot = new src_1.Account({
            poseidon,
            seed: bytes_1.bs58.encode(new Uint8Array(32).fill(3)),
        });
        recipientAccount = src_1.Account.fromPubkey(recipientAccountRoot.getPublicKey(), poseidon);
        (0, src_1.createOutUtxos)({
            publicMint: tokenCtx.mint,
            publicAmountSpl: splAmount,
            inUtxos: [utxo1, utxoSol],
            publicAmountSol: src_1.BN_0,
            changeUtxoAccount: k0,
            action: src_1.Action.UNSHIELD,
            poseidon,
            numberMaxOutUtxos,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("NO_POSEIDON_HASHER_PROVIDED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                publicAmountSol: src_1.BN_0,
                // poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
            functionName: "createOutUtxos",
        });
    });
    (0, mocha_1.it)("INVALID_NUMBER_OF_RECIPIENTS", async () => {
        (0, chai_1.expect)(() => {
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                publicAmountSol: src_1.BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
                outUtxos: [
                    new src_1.Utxo({
                        poseidon,
                        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                    }),
                    new src_1.Utxo({
                        poseidon,
                        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                    }),
                ],
                numberMaxOutUtxos,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
            functionName: "createOutUtxos",
        });
    });
    (0, mocha_1.it)("INVALID_RECIPIENT_MINT", async () => {
        let invalidMint = web3_js_1.Keypair.generate().publicKey;
        (0, chai_1.expect)(() => {
            // @ts-ignore
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                publicAmountSol: src_1.BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
                outUtxos: [
                    new src_1.Utxo({
                        poseidon,
                        assets: [web3_js_1.SystemProgram.programId, invalidMint],
                        amounts: [src_1.BN_0, src_1.BN_1],
                        assetLookupTable: [
                            ...lightProvider.lookUpTables.assetLookupTable,
                            ...[invalidMint.toBase58()],
                        ],
                        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                    }),
                ],
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
            functionName: "createOutUtxos",
        });
    });
    (0, mocha_1.it)("RECIPIENTS_SUM_AMOUNT_MISMATCH", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                publicAmountSol: src_1.BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
                outUtxos: [
                    new src_1.Utxo({
                        poseidon,
                        assets: [web3_js_1.SystemProgram.programId, utxo1.assets[1]],
                        amounts: [src_1.BN_0, new anchor_1.BN(1e12)],
                        assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                    }),
                ],
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH,
            functionName: "validateUtxoAmounts",
        });
    });
    (0, mocha_1.it)("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                // publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                // publicAmountSol: BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
            functionName: "createOutUtxos",
        });
    });
    (0, mocha_1.it)("NO_PUBLIC_MINT_PROVIDED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            (0, src_1.createOutUtxos)({
                // publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol],
                // publicAmountSol: BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
                relayerFee: src_1.BN_1,
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
            functionName: "createOutUtxos",
        });
    });
    // it("SPL_AMOUNT_UNDEFINED",async () => {
    //     expect(()=>{
    //         // @ts-ignore
    //         createOutUtxos({
    //             publicMint: tokenCtx.mint,
    //             publicAmountSpl: splAmount,
    //             inUtxos: [utxo1, utxoSol],
    //             publicAmountSol: BN_0,
    //             poseidon,
    //             changeUtxoAccount: k0,
    //             action: Action.UNSHIELD,
    //             // @ts-ignore
    //             recipients: [{account: recipientAccount, mint: utxo1.assets[1], solAmount: BN_0}],
    //         });
    //     }).to.throw(CreateUtxoError).includes({
    //         code: CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,
    //         functionName: "createOutUtxos"
    //     })
    // })
    (0, mocha_1.it)("INVALID_OUTPUT_UTXO_LENGTH", async () => {
        let invalidMint = web3_js_1.Keypair.generate().publicKey;
        let utxoSol0 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, invalidMint],
            amounts: [new anchor_1.BN(1e6), new anchor_1.BN(1e6)],
            assetLookupTable: [
                ...lightProvider.lookUpTables.assetLookupTable,
                ...[invalidMint.toBase58()],
            ],
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl: splAmount,
                inUtxos: [utxo1, utxoSol0],
                publicAmountSol: src_1.BN_0,
                poseidon,
                changeUtxoAccount: k0,
                action: src_1.Action.UNSHIELD,
                outUtxos: [
                    new src_1.Utxo({
                        poseidon,
                        assets: [web3_js_1.SystemProgram.programId, utxo1.assets[1]],
                        amounts: [src_1.BN_0, src_1.BN_1],
                        assetLookupTable: [
                            ...lightProvider.lookUpTables.assetLookupTable,
                            ...[invalidMint.toBase58()],
                        ],
                        verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                    }),
                ],
                numberMaxOutUtxos,
                assetLookupTable: [
                    ...lightProvider.lookUpTables.assetLookupTable,
                    ...[invalidMint.toBase58()],
                ],
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.CreateUtxoError)
            .includes({
            code: src_1.CreateUtxoErrorCode.INVALID_OUTPUT_UTXO_LENGTH,
            functionName: "createOutUtxos",
        });
    });
});
//# sourceMappingURL=createOutUtxos.test.js.map