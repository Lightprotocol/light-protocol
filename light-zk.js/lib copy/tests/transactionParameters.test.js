"use strict";
//@ts-nocheck
Object.defineProperty(exports, "__esModule", { value: true });
const chai_1 = require("chai");
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const src_1 = require("../src");
const spl_token_1 = require("@solana/spl-token");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const VERIFIER_IDLS = [
    src_1.IDL_VERIFIER_PROGRAM_ZERO,
    src_1.IDL_VERIFIER_PROGRAM_ONE,
    src_1.IDL_VERIFIER_PROGRAM_TWO,
];
describe("Transaction Parameters Functional", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey1 = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey2 = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
    let poseidon, lightProvider, deposit_utxo1, relayer, keypair;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        lightProvider = await src_1.Provider.loadMock();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey3, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    // TODO(vadorovsky): This test fails because of insufficient size of the
    // borsh buffer. Once we are closer to implementing multisig, we need to fix
    // that problem properly.
    mocha_1.it.skip("Serialization Transfer Functional", async () => {
        let outputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [
                new anchor_1.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
                new anchor_1.BN(depositAmount),
            ],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let j = 0;
        const inputUtxos = [deposit_utxo1];
        const outputUtxos = [outputUtxo];
        const paramsOriginal = new src_1.TransactionParameters({
            inputUtxos,
            outputUtxos,
            eventMerkleTreePubkey: src_1.MerkleTreeConfig.getEventMerkleTreePda(),
            transactionMerkleTreePubkey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            poseidon,
            action: src_1.Action.TRANSFER,
            relayer,
            verifierIdl: VERIFIER_IDLS[j],
        });
        let bytes = await paramsOriginal.toBytes();
        let params = await src_1.TransactionParameters.fromBytes({
            poseidon,
            utxoIdls: [src_1.IDL_VERIFIER_PROGRAM_ZERO],
            relayer,
            bytes,
            verifierIdl: VERIFIER_IDLS[j],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        chai_1.assert.equal(params.action.toString(), src_1.Action.TRANSFER.toString());
        chai_1.assert.equal(params.publicAmountSpl.toString(), "0");
        chai_1.assert.equal(params.publicAmountSol.sub(src_1.FIELD_SIZE).mul(new anchor_1.BN(-1)).toString(), relayer.getRelayerFee().toString());
        chai_1.assert.equal(params.assetPubkeys[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(params.assetPubkeys[1].toBase58(), src_1.MINT.toBase58());
        chai_1.assert.equal(params.assetPubkeys[2].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(params.accounts.recipientSpl?.toBase58(), src_1.AUTHORITY.toBase58());
        chai_1.assert.equal(params.accounts.recipientSol?.toBase58(), src_1.AUTHORITY.toBase58());
        chai_1.assert.equal(params.accounts.transactionMerkleTree.toBase58(), src_1.MerkleTreeConfig.getTransactionMerkleTreePda().toBase58());
        chai_1.assert.equal(params.accounts.verifierState, undefined);
        chai_1.assert.equal(params.accounts.programMerkleTree, src_1.merkleTreeProgramId);
        chai_1.assert.equal(params.accounts.signingAddress?.toBase58(), relayer.accounts.relayerPubkey.toBase58());
        chai_1.assert.equal(params.accounts.signingAddress?.toBase58(), params.relayer.accounts.relayerPubkey.toBase58());
        chai_1.assert.equal(params.accounts.authority.toBase58(), src_1.Transaction.getSignerAuthorityPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
        chai_1.assert.equal(params.accounts.registeredVerifierPda.toBase58(), src_1.Transaction.getRegisteredVerifierPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
        chai_1.assert.equal(params.accounts.systemProgramId, web3_js_1.SystemProgram.programId);
        chai_1.assert.equal(params.accounts.tokenProgram, spl_token_1.TOKEN_PROGRAM_ID);
        chai_1.assert.equal(params.accounts.tokenAuthority?.toBase58(), src_1.Transaction.getTokenAuthority().toBase58());
        chai_1.assert.equal(src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in.toString(), src_1.TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString());
        chai_1.assert.equal(params.inputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in);
        chai_1.assert.equal(params.outputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).out);
        for (let i in inputUtxos) {
            chai_1.assert.equal(params.inputUtxos[i].getCommitment(poseidon), inputUtxos[i].getCommitment(poseidon));
        }
        for (let i in outputUtxos) {
            chai_1.assert.equal(params.outputUtxos[i].getCommitment(poseidon), outputUtxos[i].getCommitment(poseidon));
        }
    });
    (0, mocha_1.it)("Transfer Functional", async () => {
        let outputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [
                new anchor_1.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
                new anchor_1.BN(depositAmount),
            ],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let j in VERIFIER_IDLS) {
            const inputUtxos = [deposit_utxo1];
            const outputUtxos = [outputUtxo];
            const params = new src_1.TransactionParameters({
                inputUtxos,
                outputUtxos,
                eventMerkleTreePubkey: mockPubkey2,
                transactionMerkleTreePubkey: mockPubkey2,
                poseidon,
                action: src_1.Action.TRANSFER,
                relayer,
                verifierIdl: VERIFIER_IDLS[j],
            });
            chai_1.assert.equal(params.action.toString(), src_1.Action.TRANSFER.toString());
            chai_1.assert.equal(params.publicAmountSpl.toString(), "0");
            chai_1.assert.equal(params.publicAmountSol.sub(src_1.FIELD_SIZE).mul(new anchor_1.BN(-1)).toString(), relayer.getRelayerFee().toString());
            chai_1.assert.equal(params.assetPubkeys[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.assetPubkeys[1].toBase58(), src_1.MINT.toBase58());
            chai_1.assert.equal(params.assetPubkeys[2].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.accounts.recipientSpl?.toBase58(), src_1.AUTHORITY.toBase58());
            chai_1.assert.equal(params.accounts.recipientSol?.toBase58(), src_1.AUTHORITY.toBase58());
            chai_1.assert.equal(params.accounts.transactionMerkleTree.toBase58(), mockPubkey2.toBase58());
            chai_1.assert.equal(params.accounts.verifierState, undefined);
            chai_1.assert.equal(params.accounts.programMerkleTree, src_1.merkleTreeProgramId);
            chai_1.assert.equal(params.accounts.signingAddress, relayer.accounts.relayerPubkey);
            chai_1.assert.equal(params.accounts.signingAddress, params.relayer.accounts.relayerPubkey);
            chai_1.assert.equal(params.accounts.authority.toBase58(), src_1.Transaction.getSignerAuthorityPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.registeredVerifierPda.toBase58(), src_1.Transaction.getRegisteredVerifierPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.systemProgramId, web3_js_1.SystemProgram.programId);
            chai_1.assert.equal(params.accounts.tokenProgram, spl_token_1.TOKEN_PROGRAM_ID);
            chai_1.assert.equal(params.accounts.tokenAuthority?.toBase58(), src_1.Transaction.getTokenAuthority().toBase58());
            chai_1.assert.equal(src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in.toString(), src_1.TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString());
            chai_1.assert.equal(params.inputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in);
            chai_1.assert.equal(params.outputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).out);
            for (let i in inputUtxos) {
                chai_1.assert.equal(params.inputUtxos[i].getCommitment(poseidon), inputUtxos[i].getCommitment(poseidon));
            }
            for (let i in outputUtxos) {
                chai_1.assert.equal(params.outputUtxos[i].getCommitment(poseidon), outputUtxos[i].getCommitment(poseidon));
            }
        }
    });
    (0, mocha_1.it)("Deposit Functional", async () => {
        for (let j in VERIFIER_IDLS) {
            const outputUtxos = [deposit_utxo1];
            const params = new src_1.TransactionParameters({
                outputUtxos,
                eventMerkleTreePubkey: mockPubkey2,
                transactionMerkleTreePubkey: mockPubkey2,
                senderSpl: mockPubkey,
                senderSol: mockPubkey1,
                poseidon,
                action: src_1.Action.SHIELD,
                verifierIdl: VERIFIER_IDLS[j],
            });
            chai_1.assert.equal(params.publicAmountSpl.toString(), depositAmount.toString());
            chai_1.assert.equal(params.publicAmountSol.toString(), depositFeeAmount.toString());
            chai_1.assert.equal(params.assetPubkeys[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.assetPubkeys[1].toBase58(), src_1.MINT.toBase58());
            chai_1.assert.equal(params.assetPubkeys[2].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.accounts.senderSpl?.toBase58(), mockPubkey.toBase58());
            chai_1.assert.equal(params.accounts.senderSol?.toBase58(), src_1.TransactionParameters.getEscrowPda(src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.transactionMerkleTree.toBase58(), mockPubkey2.toBase58());
            chai_1.assert.equal(params.accounts.verifierState, undefined);
            chai_1.assert.equal(params.accounts.programMerkleTree, src_1.merkleTreeProgramId);
            chai_1.assert.equal(params.accounts.signingAddress, mockPubkey1);
            chai_1.assert.equal(params.accounts.signingAddress, params.relayer.accounts.relayerPubkey);
            chai_1.assert.equal(params.accounts.authority.toBase58(), src_1.Transaction.getSignerAuthorityPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.registeredVerifierPda.toBase58(), src_1.Transaction.getRegisteredVerifierPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.systemProgramId, web3_js_1.SystemProgram.programId);
            chai_1.assert.equal(params.accounts.tokenProgram, spl_token_1.TOKEN_PROGRAM_ID);
            chai_1.assert.equal(params.accounts.tokenAuthority?.toBase58(), src_1.Transaction.getTokenAuthority().toBase58());
            chai_1.assert.equal(src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in.toString(), src_1.TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString());
            chai_1.assert.equal(params.action.toString(), src_1.Action.SHIELD.toString());
            chai_1.assert.equal(params.inputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in);
            chai_1.assert.equal(params.outputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).out);
            for (let i in outputUtxos) {
                chai_1.assert.equal(params.outputUtxos[i].getCommitment(poseidon), outputUtxos[i].getCommitment(poseidon));
            }
        }
    });
    (0, mocha_1.it)("Withdrawal Functional", async () => {
        for (let j in VERIFIER_IDLS) {
            const inputUtxos = [deposit_utxo1];
            const params = new src_1.TransactionParameters({
                inputUtxos,
                eventMerkleTreePubkey: mockPubkey2,
                transactionMerkleTreePubkey: mockPubkey2,
                recipientSpl: mockPubkey,
                recipientSol: mockPubkey1,
                poseidon,
                action: src_1.Action.UNSHIELD,
                relayer,
                verifierIdl: VERIFIER_IDLS[j],
            });
            chai_1.assert.equal(params.action.toString(), src_1.Action.UNSHIELD.toString());
            chai_1.assert.equal(params.publicAmountSpl.sub(src_1.FIELD_SIZE).mul(new anchor_1.BN(-1)).toString(), depositAmount.toString());
            chai_1.assert.equal(params.publicAmountSol.sub(src_1.FIELD_SIZE).mul(new anchor_1.BN(-1)).toString(), depositFeeAmount.toString());
            chai_1.assert.equal(params.assetPubkeys[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.assetPubkeys[1].toBase58(), src_1.MINT.toBase58());
            chai_1.assert.equal(params.assetPubkeys[2].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
            chai_1.assert.equal(params.accounts.recipientSpl?.toBase58(), mockPubkey.toBase58());
            chai_1.assert.equal(params.accounts.recipientSol?.toBase58(), mockPubkey1.toBase58());
            chai_1.assert.equal(params.accounts.transactionMerkleTree.toBase58(), mockPubkey2.toBase58());
            chai_1.assert.equal(params.accounts.verifierState, undefined);
            chai_1.assert.equal(params.accounts.programMerkleTree, src_1.merkleTreeProgramId);
            chai_1.assert.equal(params.accounts.signingAddress, relayer.accounts.relayerPubkey);
            chai_1.assert.equal(params.accounts.signingAddress, params.relayer.accounts.relayerPubkey);
            chai_1.assert.equal(params.accounts.authority.toBase58(), src_1.Transaction.getSignerAuthorityPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.registeredVerifierPda.toBase58(), src_1.Transaction.getRegisteredVerifierPda(src_1.merkleTreeProgramId, src_1.TransactionParameters.getVerifierProgramId(VERIFIER_IDLS[j])).toBase58());
            chai_1.assert.equal(params.accounts.systemProgramId, web3_js_1.SystemProgram.programId);
            chai_1.assert.equal(params.accounts.tokenProgram, spl_token_1.TOKEN_PROGRAM_ID);
            chai_1.assert.equal(params.accounts.tokenAuthority?.toBase58(), src_1.Transaction.getTokenAuthority().toBase58());
            chai_1.assert.equal(src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in.toString(), src_1.TransactionParameters.getVerifierConfig(VERIFIER_IDLS[j]).in.toString());
            chai_1.assert.equal(params.inputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).in);
            chai_1.assert.equal(params.outputUtxos.length, src_1.TransactionParameters.getVerifierConfig(params.verifierIdl).out);
            for (let i in inputUtxos) {
                chai_1.assert.equal(params.inputUtxos[i].getCommitment(poseidon), inputUtxos[i].getCommitment(poseidon));
            }
        }
    });
});
describe("Test TransactionParameters Methods", () => {
    let lightProvider;
    (0, mocha_1.it)("Test getAssetPubkeys", async () => {
        lightProvider = await src_1.Provider.loadMock();
        const poseidon = await buildPoseidonOpt();
        let inputUtxos = [
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
        ];
        let outputUtxos = [
            new src_1.Utxo({
                poseidon,
                amounts: [src_1.BN_2, new anchor_1.BN(4)],
                assets: [web3_js_1.SystemProgram.programId, src_1.MINT],
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            }),
            new src_1.Utxo({
                poseidon,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            }),
        ];
        let { assetPubkeysCircuit, assetPubkeys } = src_1.TransactionParameters.getAssetPubkeys(inputUtxos, outputUtxos);
        chai_1.assert.equal(assetPubkeys[0].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(assetPubkeys[1].toBase58(), src_1.MINT.toBase58());
        chai_1.assert.equal(assetPubkeys[2].toBase58(), web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(assetPubkeysCircuit[0].toString(), (0, src_1.hashAndTruncateToCircuit)(web3_js_1.SystemProgram.programId.toBuffer()).toString());
        chai_1.assert.equal(assetPubkeysCircuit[1].toString(), (0, src_1.hashAndTruncateToCircuit)(src_1.MINT.toBuffer()).toString());
        chai_1.assert.equal(assetPubkeysCircuit[2].toString(), "0");
    });
    (0, mocha_1.it)("Test getExtAmount", async () => {
        const poseidon = await buildPoseidonOpt();
        let inputUtxos = [
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
        ];
        let outputUtxos = [
            new src_1.Utxo({
                poseidon,
                amounts: [src_1.BN_2, new anchor_1.BN(4)],
                assets: [web3_js_1.SystemProgram.programId, src_1.MINT],
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            }),
            new src_1.Utxo({
                poseidon,
                assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
            }),
        ];
        let { assetPubkeysCircuit } = src_1.TransactionParameters.getAssetPubkeys(inputUtxos, outputUtxos);
        let publicAmountSol = src_1.TransactionParameters.getExternalAmount(0, inputUtxos, outputUtxos, assetPubkeysCircuit);
        chai_1.assert.equal(publicAmountSol.toString(), "2");
        let publicAmountSpl = src_1.TransactionParameters.getExternalAmount(1, inputUtxos, outputUtxos, assetPubkeysCircuit);
        chai_1.assert.equal(publicAmountSpl.toString(), "4");
        outputUtxos[1] = new src_1.Utxo({
            poseidon,
            amounts: [new anchor_1.BN(3), new anchor_1.BN(5)],
            assets: [web3_js_1.SystemProgram.programId, src_1.MINT],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let publicAmountSpl2Outputs = src_1.TransactionParameters.getExternalAmount(1, inputUtxos, outputUtxos, assetPubkeysCircuit);
        chai_1.assert.equal(publicAmountSpl2Outputs.toString(), "9");
        let publicAmountSol2Outputs = src_1.TransactionParameters.getExternalAmount(0, inputUtxos, outputUtxos, assetPubkeysCircuit);
        chai_1.assert.equal(publicAmountSol2Outputs.toString(), "5");
    });
});
describe("Test General TransactionParameters Errors", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
    let poseidon, lightProvider, deposit_utxo1, relayer, keypair;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey3, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("NO_UTXOS_PROVIDED", async () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.NO_UTXOS_PROVIDED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("NO_POSEIDON_HASHER_PROVIDED", async () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                // @ts-ignore:
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("NO_ACTION_PROVIDED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                // @ts-ignore:
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.NO_ACTION_PROVIDED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("NO_VERIFIER_PROVIDED", () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore:
            new src_1.TransactionParameters({
                outputUtxos: [deposit_utxo1],
                transactionMerkleTreePubkey: mockPubkey,
                senderSpl: mockPubkey,
                senderSol: mockPubkey,
                poseidon,
                action: src_1.Action.SHIELD,
            });
        })
            .to.throw(src_1.TransactionParametersError)
            .to.include({
            code: src_1.TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED,
            functionName: "constructor",
        });
    });
});
describe("Test TransactionParameters Transfer Errors", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let keypair;
    let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        outputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [
                new anchor_1.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
                new anchor_1.BN(depositAmount),
            ],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("RELAYER_UNDEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [outputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.RELAYER_UNDEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("PUBLIC_AMOUNT_SPL_NOT_ZERO", () => {
        const localOutputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount).sub(relayer.getRelayerFee()), src_1.BN_0],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [localOutputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("PUBLIC_AMOUNT_SOL_NOT_ZERO", () => {
        const localOutputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [localOutputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [outputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    recipientSpl: mockPubkey,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [outputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    recipientSol: mockPubkey,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL_SENDER_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [outputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    senderSol: mockPubkey,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SOL_SENDER_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL_SENDER_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    outputUtxos: [outputUtxo],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    poseidon,
                    action: src_1.Action.TRANSFER,
                    senderSpl: mockPubkey,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SPL_SENDER_DEFINED,
                functionName: "constructor",
            });
        }
    });
});
describe("Test TransactionParameters Deposit Errors", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let keypair;
    let poseidon, lightProvider, deposit_utxo1, relayer;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("SOL_SENDER_UNDEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.SOL_SENDER_UNDEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL_SENDER_UNDEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.SPL_SENDER_UNDEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("RELAYER_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.RELAYER_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL PUBLIC_AMOUNT_NOT_U64", () => {
        let utxo_sol_amount_no_u641 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let utxo_sol_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), src_1.BN_0],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL PUBLIC_AMOUNT_NOT_U64", () => {
        let utxo_spl_amount_no_u641 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN("18446744073709551615")],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let utxo_spl_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN("1")],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    recipientSpl: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("No senderSpl spl needed without spl amount", () => {
        let utxo_sol_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), src_1.BN_0],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            // senderSpl fee always needs to be defined because we use it as the signer
            // should work since no spl amount
            new src_1.TransactionParameters({
                outputUtxos: [utxo_sol_amount_no_u642],
                eventMerkleTreePubkey: mockPubkey,
                transactionMerkleTreePubkey: mockPubkey,
                senderSol: mockPubkey,
                poseidon,
                action: src_1.Action.SHIELD,
                verifierIdl: VERIFIER_IDLS[verifier],
            });
        }
    });
    (0, mocha_1.it)("SPL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    recipientSpl: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL_RECIPIENT_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    outputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    senderSol: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.SHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
                functionName: "constructor",
            });
        }
    });
});
describe("Test TransactionParameters Withdrawal Errors", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let keypair;
    let poseidon, lightProvider, deposit_utxo1, outputUtxo, relayer;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        outputUtxo = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [
                new anchor_1.BN(depositFeeAmount).sub(relayer.getRelayerFee()),
                new anchor_1.BN(depositAmount),
            ],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
    });
    (0, mocha_1.it)("SOL_RECIPIENT_UNDEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    recipientSpl: mockPubkey,
                    // senderSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("RELAYER_UNDEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    recipientSpl: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionErrorCode.RELAYER_UNDEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL PUBLIC_AMOUNT_NOT_U64", () => {
        let utxo_sol_amount_no_u641 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), new anchor_1.BN(depositAmount)],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let utxo_sol_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), src_1.BN_0],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [utxo_sol_amount_no_u641, utxo_sol_amount_no_u642],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    recipientSpl: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL PUBLIC_AMOUNT_NOT_U64", () => {
        let utxo_spl_amount_no_u641 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN("18446744073709551615")],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let utxo_spl_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN("1")],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [utxo_spl_amount_no_u641, utxo_spl_amount_no_u642],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    recipientSpl: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SOL_SENDER_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSol: mockPubkey,
                    recipientSpl: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SOL_SENDER_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("SPL_SENDER_DEFINED", () => {
        for (let verifier in VERIFIER_IDLS) {
            (0, chai_1.expect)(() => {
                new src_1.TransactionParameters({
                    inputUtxos: [deposit_utxo1],
                    eventMerkleTreePubkey: mockPubkey,
                    transactionMerkleTreePubkey: mockPubkey,
                    senderSpl: mockPubkey,
                    recipientSpl: mockPubkey,
                    recipientSol: mockPubkey,
                    poseidon,
                    action: src_1.Action.UNSHIELD,
                    relayer,
                    verifierIdl: VERIFIER_IDLS[verifier],
                });
            })
                .to.throw(src_1.TransactionParametersError)
                .to.include({
                code: src_1.TransactionParametersErrorCode.SPL_SENDER_DEFINED,
                functionName: "constructor",
            });
        }
    });
    (0, mocha_1.it)("no recipientSpl spl should work since no spl amount", () => {
        let utxo_sol_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN("18446744073709551615"), src_1.BN_0],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            // should work since no spl amount
            new src_1.TransactionParameters({
                inputUtxos: [utxo_sol_amount_no_u642],
                eventMerkleTreePubkey: mockPubkey,
                transactionMerkleTreePubkey: mockPubkey,
                recipientSol: mockPubkey,
                poseidon,
                action: src_1.Action.UNSHIELD,
                relayer,
                verifierIdl: VERIFIER_IDLS[verifier],
            });
        }
    });
    (0, mocha_1.it)("no recipientSpl sol should work since no sol amount", () => {
        let utxo_sol_amount_no_u642 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, new anchor_1.BN("18446744073709551615")],
            account: keypair,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        for (let verifier in VERIFIER_IDLS) {
            // should work since no sol amount
            new src_1.TransactionParameters({
                inputUtxos: [utxo_sol_amount_no_u642],
                eventMerkleTreePubkey: mockPubkey,
                transactionMerkleTreePubkey: mockPubkey,
                recipientSpl: mockPubkey,
                poseidon,
                action: src_1.Action.UNSHIELD,
                relayer,
                verifierIdl: VERIFIER_IDLS[verifier],
            });
        }
    });
});
//# sourceMappingURL=transactionParameters.test.js.map