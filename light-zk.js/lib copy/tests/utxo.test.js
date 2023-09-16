"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
//@ts-nocheck
const chai_1 = require("chai");
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const tmp_test_psp_1 = require("./testData/tmp_test_psp");
const src_1 = require("../src");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("Utxo Functional", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let poseidon, lightProvider, deposit_utxo1, keypair;
    before(async () => {
        poseidon = await buildPoseidonOpt();
        // TODO: make fee mandatory
        // relayer = new Relayer(relayerMockPubKey, mockPubkey, new BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            index: 1,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("rnd utxo functional loop 100", async () => {
        for (let i = 0; i < 100; i++) {
            // try basic tests for rnd empty utxo
            const utxo4Account = new src_1.Account({ poseidon });
            const utxo4 = new src_1.Utxo({
                poseidon,
                amounts: [new anchor_1.BN(123)],
                account: utxo4Account,
                appDataHash: new anchor_1.BN(src_1.verifierProgramTwoProgramId.toBuffer()),
                includeAppData: false,
                verifierAddress: new web3_js_1.PublicKey(lightProvider.lookUpTables.verifierProgramLookupTable[1]),
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
            // toBytesProvider
            const bytes4 = await utxo4.toBytes();
            // fromBytes
            const utxo40 = src_1.Utxo.fromBytes({
                poseidon,
                bytes: bytes4,
                index: 0,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
            src_1.Utxo.equal(poseidon, utxo4, utxo40);
            // toBytes
            const bytes4Compressed = await utxo4.toBytes(true);
            // fromBytes
            const utxo40Compressed = src_1.Utxo.fromBytes({
                poseidon,
                account: utxo4Account,
                bytes: bytes4Compressed,
                index: 0,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
            src_1.Utxo.equal(poseidon, utxo4, utxo40Compressed);
            // encrypt
            const encBytes4 = await utxo4.encrypt(poseidon, src_1.MerkleTreeConfig.getTransactionMerkleTreePda());
            const encBytes41 = await utxo4.encrypt(poseidon, src_1.MerkleTreeConfig.getTransactionMerkleTreePda());
            chai_1.assert.equal(encBytes4.toString(), encBytes41.toString());
            const utxo41 = await src_1.Utxo.decrypt({
                poseidon,
                encBytes: encBytes4,
                account: utxo4Account,
                index: 0,
                merkleTreePdaPublicKey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
                commitment: new anchor_1.BN(utxo4.getCommitment(poseidon)).toArrayLike(Buffer, "le", 32),
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
            if (utxo41) {
                src_1.Utxo.equal(poseidon, utxo4, utxo41);
            }
            else {
                throw new Error("decrypt failed");
            }
        }
    });
    (0, mocha_1.it)("toString", async () => {
        const amountFee = "1";
        const amountToken = "2";
        const assetPubkey = src_1.MINT;
        const seed32 = new Uint8Array(32).fill(1).toString();
        let inputs = {
            keypair: new src_1.Account({ poseidon, seed: seed32 }),
            amountFee,
            amountToken,
            assetPubkey,
            assets: [web3_js_1.SystemProgram.programId, assetPubkey],
            amounts: [new anchor_1.BN(amountFee), new anchor_1.BN(amountToken)],
            blinding: new anchor_1.BN(new Uint8Array(31).fill(2)),
            index: 1,
        };
        let utxo0 = new src_1.Utxo({
            poseidon,
            assets: inputs.assets,
            amounts: inputs.amounts,
            account: inputs.keypair,
            blinding: inputs.blinding,
            index: inputs.index,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let string = await utxo0.toString();
        let utxo1 = src_1.Utxo.fromString(string, poseidon, lightProvider.lookUpTables.assetLookupTable, lightProvider.lookUpTables.verifierProgramLookupTable);
        // cannot compute nullifier in utxo1 because no privkey is serialized with toString()
        src_1.Utxo.equal(poseidon, utxo0, utxo1, true);
    });
    (0, mocha_1.it)("toString", async () => {
        const amountFee = "1";
        const amountToken = "2";
        const assetPubkey = src_1.MINT;
        const seed32 = new Uint8Array(32).fill(1).toString();
        let inputs = {
            keypair: new src_1.Account({ poseidon, seed: seed32 }),
            amountFee,
            amountToken,
            assetPubkey,
            assets: [web3_js_1.SystemProgram.programId, assetPubkey],
            amounts: [new anchor_1.BN(amountFee), new anchor_1.BN(amountToken)],
            blinding: new anchor_1.BN(new Uint8Array(31).fill(2)),
            index: 1,
        };
        let utxo0 = new src_1.Utxo({
            poseidon,
            assets: inputs.assets,
            amounts: inputs.amounts,
            account: inputs.keypair,
            blinding: inputs.blinding,
            index: inputs.index,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let string = await utxo0.toString();
        let utxo1 = src_1.Utxo.fromString(string, poseidon, lightProvider.lookUpTables.assetLookupTable, lightProvider.lookUpTables.verifierProgramLookupTable);
        // cannot compute nullifier in utxo1 because no privkey is serialized with toString()
        src_1.Utxo.equal(poseidon, utxo0, utxo1, true);
    });
    (0, mocha_1.it)("encryption", async () => {
        const amountFee = "1";
        const amountToken = "2";
        const assetPubkey = src_1.MINT;
        const seed32 = new Uint8Array(32).fill(1).toString();
        let inputs = {
            keypair: new src_1.Account({ poseidon, seed: seed32 }),
            amountFee,
            amountToken,
            assetPubkey,
            assets: [web3_js_1.SystemProgram.programId, assetPubkey],
            amounts: [new anchor_1.BN(amountFee), new anchor_1.BN(amountToken)],
            blinding: new anchor_1.BN(new Uint8Array(31).fill(2)),
            index: 1,
        };
        let utxo0 = new src_1.Utxo({
            poseidon,
            assets: inputs.assets,
            amounts: inputs.amounts,
            account: inputs.keypair,
            blinding: inputs.blinding,
            index: inputs.index,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        // functional
        chai_1.assert.equal(utxo0.amounts[0].toString(), amountFee);
        chai_1.assert.equal(utxo0.amounts[1].toString(), amountToken);
        chai_1.assert.equal(utxo0.assets[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());
        chai_1.assert.equal(utxo0.assetsCircuit[0].toString(), (0, src_1.hashAndTruncateToCircuit)(web3_js_1.SystemProgram.programId.toBytes()).toString());
        chai_1.assert.equal(utxo0.assetsCircuit[1].toString(), (0, src_1.hashAndTruncateToCircuit)(assetPubkey.toBytes()).toString());
        chai_1.assert.equal(utxo0.appDataHash.toString(), "0");
        chai_1.assert.equal(utxo0.poolType.toString(), "0");
        chai_1.assert.equal(utxo0.verifierAddress.toString(), web3_js_1.SystemProgram.programId.toString());
        chai_1.assert.equal(utxo0.verifierAddressCircuit.toString(), "0");
        chai_1.assert.equal(utxo0.getCommitment(poseidon)?.toString(), "8291567517196483063353958025601041123319055074768288393371971758484371715486");
        chai_1.assert.equal(utxo0.getNullifier(poseidon)?.toString(), "6203060337570741528902613554275892537213176828384528961609701446906034353029");
        // toBytes
        const bytes = await utxo0.toBytes();
        // fromBytes
        const utxo1 = src_1.Utxo.fromBytes({
            poseidon,
            account: inputs.keypair,
            bytes,
            index: inputs.index,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        src_1.Utxo.equal(poseidon, utxo0, utxo1);
        // encrypt
        const encBytes = await utxo1.encrypt(poseidon, src_1.MerkleTreeConfig.getTransactionMerkleTreePda());
        // decrypt
        const utxo3 = await src_1.Utxo.decrypt({
            poseidon,
            encBytes,
            account: inputs.keypair,
            index: inputs.index,
            merkleTreePdaPublicKey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            commitment: new anchor_1.BN(utxo1.getCommitment(poseidon)).toArrayLike(Buffer, "le", 32),
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        if (utxo3) {
            src_1.Utxo.equal(poseidon, utxo0, utxo3);
        }
        else {
            throw new Error("decrypt failed");
        }
        let pubKey = inputs.keypair.getPublicKey();
        // encrypting with nacl because this utxo's account does not have an aes secret key since it is instantiated from a public key
        const receivingUtxo = new src_1.Utxo({
            poseidon,
            assets: inputs.assets,
            amounts: inputs.amounts,
            account: src_1.Account.fromPubkey(pubKey, poseidon),
            blinding: inputs.blinding,
            index: inputs.index,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        // encrypt
        const encBytesNacl = await receivingUtxo.encrypt(poseidon, src_1.MerkleTreeConfig.getTransactionMerkleTreePda());
        // decrypt
        const receivingUtxo1 = await src_1.Utxo.decrypt({
            poseidon,
            encBytes: encBytesNacl,
            account: inputs.keypair,
            index: inputs.index,
            merkleTreePdaPublicKey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            aes: false,
            commitment: new anchor_1.BN(receivingUtxo.getCommitment(poseidon)).toArrayLike(Buffer, "le", 32),
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        if (receivingUtxo1) {
            src_1.Utxo.equal(poseidon, receivingUtxo, receivingUtxo1, true);
        }
        else {
            throw new Error("decrypt failed");
        }
    });
    (0, mocha_1.it)("Program utxo to/from bytes ", async () => {
        const verifierProgramId = new web3_js_1.PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
        const account = new src_1.Account({
            poseidon,
            seed: bytes_1.bs58.encode(new Uint8Array(32).fill(1)),
        });
        const outputUtxo = new src_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            account,
            amounts: [new anchor_1.BN(1000000)],
            appData: { releaseSlot: src_1.BN_1 },
            appDataIdl: tmp_test_psp_1.IDL,
            verifierAddress: verifierProgramId,
            index: 0,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let bytes = await outputUtxo.toBytes();
        let utxo1 = src_1.Utxo.fromBytes({
            poseidon,
            bytes,
            index: 0,
            account,
            appDataIdl: tmp_test_psp_1.IDL,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        src_1.Utxo.equal(poseidon, outputUtxo, utxo1);
    });
    (0, mocha_1.it)("Pick app data from utxo data", () => {
        let data = (0, src_1.createAccountObject)({
            releaseSlot: 1,
            rndOtherStuff: { s: 2342 },
            o: [2, 2, src_1.BN_2],
        }, tmp_test_psp_1.IDL.accounts, "utxoAppData");
        chai_1.assert.equal(data.releaseSlot, 1);
        chai_1.assert.equal(data.currentSlot, undefined);
        chai_1.assert.equal(data.rndOtherStuff, undefined);
        chai_1.assert.equal(data.o, undefined);
        (0, chai_1.expect)(() => {
            (0, src_1.createAccountObject)({ rndOtherStuff: { s: 2342 }, o: [2, 2, src_1.BN_2] }, tmp_test_psp_1.IDL.accounts, "utxoAppData");
        }).to.throw(Error);
    });
});
describe("Utxo Errors", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let poseidon, inputs, keypair;
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = src_1.MINT;
    let lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await buildPoseidonOpt();
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        inputs = {
            keypair: new src_1.Account({ poseidon, seed: seed32 }),
            amountFee,
            amountToken,
            assetPubkey,
            assets: [web3_js_1.SystemProgram.programId, assetPubkey],
            amounts: [new anchor_1.BN(amountFee), new anchor_1.BN(amountToken)],
            blinding: new anchor_1.BN(new Uint8Array(31).fill(2)),
        };
    });
    (0, mocha_1.it)("get nullifier without index", async () => {
        let publicKey = keypair.getPublicKey();
        let account = src_1.Account.fromPubkey(publicKey, poseidon);
        let pubkeyUtxo = new src_1.Utxo({
            poseidon,
            amounts: [src_1.BN_1],
            account,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            pubkeyUtxo.getNullifier(poseidon);
        })
            .throw(src_1.UtxoError)
            .include({
            code: src_1.UtxoErrorCode.INDEX_NOT_PROVIDED,
            functionName: "getNullifier",
        });
    });
    (0, mocha_1.it)("get nullifier without private key", async () => {
        let publicKey = keypair.getPublicKey();
        let account = src_1.Account.fromPubkey(publicKey, poseidon);
        let pubkeyUtxo = new src_1.Utxo({
            poseidon,
            amounts: [src_1.BN_1],
            account,
            index: 1,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        (0, chai_1.expect)(() => {
            pubkeyUtxo.getNullifier(poseidon);
        })
            .throw(src_1.UtxoError)
            .include({
            code: src_1.UtxoErrorCode.ACCOUNT_HAS_NO_PRIVKEY,
            functionName: "getNullifier",
        });
    });
    (0, mocha_1.it)("INVALID_ASSET_OR_AMOUNTS_LENGTH", () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: [inputs.assets[1]],
                amounts: inputs.amounts,
                account: inputs.keypair,
                blinding: inputs.blinding,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH,
            codeMessage: "Length mismatch assets: 1 != amounts: 2",
        });
    });
    (0, mocha_1.it)("EXCEEDED_MAX_ASSETS", () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: [src_1.MINT, src_1.MINT, src_1.MINT],
                amounts: [src_1.BN_1, src_1.BN_1, src_1.BN_1],
                account: inputs.keypair,
                blinding: inputs.blinding,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.EXCEEDED_MAX_ASSETS,
            codeMessage: "assets.length 3 > N_ASSETS 2",
        });
    });
    (0, mocha_1.it)("NEGATIVE_AMOUNT", () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: inputs.assets,
                amounts: [inputs.amounts[0], new anchor_1.BN(-1)],
                account: inputs.keypair,
                blinding: inputs.blinding,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.NEGATIVE_AMOUNT,
            codeMessage: "amount cannot be negative, amounts[1] = -1",
        });
    });
    (0, mocha_1.it)("APP_DATA_IDL_UNDEFINED", () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: inputs.assets,
                amounts: inputs.amounts,
                account: inputs.keypair,
                blinding: inputs.blinding,
                appData: new Array(32).fill(1),
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.APP_DATA_IDL_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("ASSET_NOT_FOUND", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: [web3_js_1.SystemProgram.programId, web3_js_1.Keypair.generate().publicKey],
                amounts: inputs.amounts,
                account: inputs.keypair,
                blinding: inputs.blinding,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.ASSET_NOT_FOUND,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("BLINDING_EXCEEDS_FIELD_SIZE", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Utxo({
                poseidon,
                assets: [web3_js_1.SystemProgram.programId, web3_js_1.Keypair.generate().publicKey],
                amounts: inputs.amounts,
                account: inputs.keypair,
                blinding: new anchor_1.BN(src_1.FIELD_SIZE),
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            });
        })
            .to.throw(src_1.UtxoError)
            .to.include({
            code: src_1.UtxoErrorCode.BLINDING_EXCEEDS_FIELD_SIZE,
            functionName: "constructor",
        });
    });
});
//# sourceMappingURL=utxo.test.js.map