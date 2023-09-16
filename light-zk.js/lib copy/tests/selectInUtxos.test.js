"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
//@ts-nocheck
const chai_1 = require("chai");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const anchor_1 = require("@coral-xyz/anchor");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const web3_js_1 = require("@solana/web3.js");
const anchor = __importStar(require("@coral-xyz/anchor"));
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildBabyjub, buildEddsa } = circomlibjs;
const src_1 = require("../src");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
const numberMaxInUtxos = 2;
const numberMaxOutUtxos = 2;
// TODO: add more tests with different numbers of utxos
// TODO: add a randomized test
describe("Test selectInUtxos Functional", () => {
    let poseidon, eddsa, babyJub, F;
    let splAmount, solAmount, token, tokenCtx, utxo1, utxo2, relayerFee, utxoSol, utxoSolBurner, utxo2Burner, utxo1Burner, recipientAccount;
    let lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await circomlibjs.buildPoseidonOpt();
        eddsa = await buildEddsa();
        babyJub = await buildBabyjub();
        F = babyJub.F;
        utxo1Burner = new src_1.Account({ poseidon, seed: seed32 });
        utxo2Burner = src_1.Account.createBurner(poseidon, seed32, new anchor.BN("0"));
        utxoSolBurner = src_1.Account.createBurner(poseidon, seed32, new anchor.BN("1"));
        splAmount = new anchor_1.BN(3);
        solAmount = new anchor_1.BN(1e6);
        token = "USDC";
        tokenCtx = src_1.TOKEN_REGISTRY.get(token);
        if (!tokenCtx)
            throw new Error("Token not supported!");
        splAmount = splAmount.mul(new anchor_1.BN(tokenCtx.decimals));
        utxo1 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e6), new anchor_1.BN(6 * tokenCtx.decimals.toNumber())],
            index: 0,
            account: utxo1Burner,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxo2 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e6), new anchor_1.BN(5 * tokenCtx.decimals.toNumber())],
            index: 0,
            account: utxo2Burner,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxoSol = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            amounts: [new anchor_1.BN(1e8)],
            index: 1,
            account: utxoSolBurner,
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
    (0, mocha_1.it)("Unshield select spl", async () => {
        const inUtxos = [utxo1, utxoSol];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            publicMint: utxo1.assets[1],
            relayerFee: src_1.RELAYER_FEE,
            publicAmountSpl: src_1.BN_1,
            poseidon,
            utxos: inUtxos,
            action: src_1.Action.UNSHIELD,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
    });
    (0, mocha_1.it)("Unshield select sol", async () => {
        const inUtxos = [utxoSol, utxo1];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            relayerFee: src_1.RELAYER_FEE,
            publicAmountSol: new anchor_1.BN(1e7),
            poseidon,
            action: src_1.Action.UNSHIELD,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxoSol);
        chai_1.assert.equal(src_1.selectInUtxos.length, 1);
    });
    (0, mocha_1.it)("UNSHIELD select sol & spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.UNSHIELD,
            relayerFee: src_1.RELAYER_FEE,
            poseidon,
            publicMint: utxo1.assets[1],
            publicAmountSol: new anchor_1.BN(1e7),
            publicAmountSpl: src_1.BN_1,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[1], utxoSol);
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
    });
    (0, mocha_1.it)("Transfer select sol & spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: new anchor_1.BN(1e7),
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.TRANSFER,
            relayerFee: src_1.RELAYER_FEE,
            poseidon,
            outUtxos,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[1], utxoSol);
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
    });
    (0, mocha_1.it)("Transfer select sol", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: new anchor_1.BN(1e7),
                    splAmount: src_1.BN_0,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.TRANSFER,
            relayerFee: src_1.RELAYER_FEE,
            poseidon,
            outUtxos,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxoSol);
        src_1.Utxo.equal(poseidon, selectedUtxo[1], utxo1);
    });
    (0, mocha_1.it)("Transfer select spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: src_1.BN_0,
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.TRANSFER,
            relayerFee: src_1.RELAYER_FEE,
            poseidon,
            outUtxos,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
    });
    (0, mocha_1.it)("Shield select sol & spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.SHIELD,
            publicMint: utxo1.assets[1],
            publicAmountSol: new anchor_1.BN(1e7),
            poseidon,
            publicAmountSpl: src_1.BN_1,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
    });
    (0, mocha_1.it)("Shield select sol", async () => {
        const inUtxos = [utxoSol, utxo1];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.SHIELD,
            poseidon,
            publicAmountSol: new anchor_1.BN(1e7),
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxoSol);
        src_1.Utxo.equal(poseidon, selectedUtxo[1], utxo1);
    });
    (0, mocha_1.it)("Shield select spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.SHIELD,
            publicMint: utxo1.assets[1],
            poseidon,
            publicAmountSpl: src_1.BN_1,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
        chai_1.assert.equal(selectedUtxo.length, 1);
    });
    (0, mocha_1.it)("3 utxos spl & sol", async () => {
        const inUtxos = [utxoSol, utxo1, utxo2];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: utxo2.amounts[0],
                    splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let selectedUtxo = (0, src_1.selectInUtxos)({
            utxos: inUtxos,
            action: src_1.Action.TRANSFER,
            relayerFee: src_1.RELAYER_FEE,
            poseidon,
            outUtxos,
            numberMaxInUtxos,
            numberMaxOutUtxos,
        });
        src_1.Utxo.equal(poseidon, selectedUtxo[0], utxo1);
        src_1.Utxo.equal(poseidon, selectedUtxo[1], utxo2);
    });
});
describe("Test selectInUtxos Errors", () => {
    let poseidon, eddsa, babyJub, F, k0, k00, kBurner;
    let splAmount, solAmount, token, tokenCtx, utxo1, utxo2, relayerFee, utxoSol, recipientAccount, lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await circomlibjs.buildPoseidonOpt();
        eddsa = await buildEddsa();
        babyJub = await buildBabyjub();
        F = babyJub.F;
        k0 = new src_1.Account({ poseidon, seed: seed32 });
        k00 = new src_1.Account({ poseidon, seed: seed32 });
        kBurner = src_1.Account.createBurner(poseidon, seed32, new anchor.BN("0"));
        splAmount = new anchor_1.BN(3);
        solAmount = new anchor_1.BN(1e6);
        token = "USDC";
        tokenCtx = src_1.TOKEN_REGISTRY.get(token);
        if (!tokenCtx)
            throw new Error("Token not supported!");
        splAmount = splAmount.mul(new anchor_1.BN(tokenCtx.decimals));
        utxo1 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e6), new anchor_1.BN(5 * tokenCtx.decimals.toNumber())],
            index: 0,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxo2 = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, tokenCtx.mint],
            amounts: [new anchor_1.BN(1e6), new anchor_1.BN(5 * tokenCtx.decimals.toNumber())],
            index: 0,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        utxoSol = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            amounts: [new anchor_1.BN(1e8)],
            index: 1,
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
    (0, mocha_1.it)("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: new anchor_1.BN(1e7),
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.UNSHIELD,
                poseidon,
                outUtxos,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("NO_PUBLIC_MINT_PROVIDED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.UNSHIELD,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                // publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                publicAmountSpl: src_1.BN_1,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("PUBLIC_SPL_AMOUNT_UNDEFINED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.UNSHIELD,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.CreateUtxoErrorCode.PUBLIC_SPL_AMOUNT_UNDEFINED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("RELAYER_FEE_UNDEFINED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.UNSHIELD,
                // relayerFee: RELAYER_FEE,
                poseidon,
                publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                publicAmountSpl: src_1.BN_1,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.RelayerErrorCode.RELAYER_FEE_UNDEFINED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("RELAYER_FEE_UNDEFINED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                // relayerFee: RELAYER_FEE,
                poseidon,
                publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                publicAmountSpl: src_1.BN_1,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.RelayerErrorCode.RELAYER_FEE_UNDEFINED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("RELAYER_FEE_DEFINED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.SHIELD,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                publicAmountSpl: src_1.BN_1,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.CreateUtxoErrorCode.RELAYER_FEE_DEFINED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("NO_UTXOS_PROVIDED", async () => {
        const inUtxos = [utxoSol, utxo1];
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                // utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                publicMint: utxo1.assets[1],
                publicAmountSol: new anchor_1.BN(1e7),
                publicAmountSpl: src_1.BN_1,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.TransactionErrorCode.NO_UTXOS_PROVIDED,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("INVALID_NUMBER_OF_RECIPIENTS", async () => {
        const mint = web3_js_1.Keypair.generate().publicKey;
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: new anchor_1.BN(1e7),
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
                {
                    mint,
                    solAmount: new anchor_1.BN(1e7),
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: [
                ...lightProvider.lookUpTables.assetLookupTable,
                ...[mint.toBase58()],
            ],
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                outUtxos,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("FAILED_TO_FIND_UTXO_COMBINATION sol", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: new anchor_1.BN(2e10),
                    splAmount: src_1.BN_1,
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                outUtxos,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("FAILED_TO_FIND_UTXO_COMBINATION spl", async () => {
        const inUtxos = [utxoSol, utxo1];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: src_1.BN_0,
                    splAmount: new anchor_1.BN(1e10),
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                outUtxos,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
            functionName: "selectInUtxos",
        });
    });
    (0, mocha_1.it)("FAILED_TO_FIND_UTXO_COMBINATION spl & sol", async () => {
        const inUtxos = [utxoSol, utxo1, utxo2];
        const outUtxos = (0, src_1.createRecipientUtxos)({
            recipients: [
                {
                    mint: utxo1.assets[1],
                    solAmount: utxo2.amounts[0].add(utxo1.amounts[0]),
                    splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
                    account: new src_1.Account({ poseidon }),
                },
            ],
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            (0, src_1.selectInUtxos)({
                utxos: inUtxos,
                action: src_1.Action.TRANSFER,
                relayerFee: src_1.RELAYER_FEE,
                poseidon,
                outUtxos,
                numberMaxInUtxos,
                numberMaxOutUtxos,
            });
        })
            .to.throw(src_1.SelectInUtxosError)
            .includes({
            code: src_1.SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
            functionName: "selectInUtxos",
        });
    });
});
//# sourceMappingURL=selectInUtxos.test.js.map