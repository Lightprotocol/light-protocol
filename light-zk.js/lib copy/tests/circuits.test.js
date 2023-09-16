"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const chai_1 = require("chai");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const src_1 = require("../src");
const tmp_test_psp_1 = require("./testData/tmp_test_psp");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let account, deposit_utxo1, mockPubkey, poseidon, lightProvider, txParamsApp, txParamsPoolType, txParamsPoolTypeOut, txParamsOutApp, txParams, txParamsSol, paramsWithdrawal, appData, relayer;
let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
// TODO: check more specific errors in tests
describe("Masp circuit tests", () => {
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await buildPoseidonOpt();
        account = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        await account.getEddsaPublicKey();
        let depositAmount = 20000;
        let depositFeeAmount = 10000;
        deposit_utxo1 = new src_1.Utxo({
            index: 0,
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: account,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        let deposit_utxoSol = new src_1.Utxo({
            index: 0,
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), src_1.BN_0],
            account: account,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        mockPubkey = web3_js_1.Keypair.generate().publicKey;
        let mockPubkey2 = web3_js_1.Keypair.generate().publicKey;
        let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
        txParams = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.SHIELD,
            poseidon,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        txParamsSol = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxoSol],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.SHIELD,
            poseidon,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        lightProvider.solMerkleTree.merkleTree = new src_1.MerkleTree(18, poseidon, [
            deposit_utxo1.getCommitment(poseidon),
            // random invalid other commitment
            poseidon.F.toString(poseidon(["123124"])),
        ]);
        chai_1.assert.equal(lightProvider.solMerkleTree?.merkleTree.indexOf(deposit_utxo1.getCommitment(poseidon)), 0);
        relayer = new src_1.Relayer(mockPubkey3, mockPubkey, new anchor_1.BN(5000));
        paramsWithdrawal = new src_1.TransactionParameters({
            inputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            recipientSpl: mockPubkey,
            recipientSol: lightProvider.wallet.publicKey,
            action: src_1.Action.UNSHIELD,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        appData = { releaseSlot: src_1.BN_1 };
        txParamsApp = new src_1.TransactionParameters({
            inputUtxos: [
                new src_1.Utxo({
                    index: 0,
                    poseidon,
                    appData,
                    appDataIdl: tmp_test_psp_1.IDL,
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                }),
            ],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.UNSHIELD,
            poseidon,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_TWO,
        });
        txParamsPoolType = new src_1.TransactionParameters({
            inputUtxos: [
                new src_1.Utxo({
                    index: 0,
                    poseidon,
                    poolType: new anchor_1.BN("12312"),
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                }),
            ],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.UNSHIELD,
            poseidon,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        txParamsPoolTypeOut = new src_1.TransactionParameters({
            outputUtxos: [
                new src_1.Utxo({
                    poseidon,
                    poolType: new anchor_1.BN("12312"),
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                }),
            ],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.UNSHIELD,
            poseidon,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        txParamsOutApp = new src_1.TransactionParameters({
            outputUtxos: [
                new src_1.Utxo({
                    poseidon,
                    appData,
                    appDataIdl: tmp_test_psp_1.IDL,
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                }),
            ],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.SHIELD,
            poseidon,
            // automatic encryption for app utxos is not implemented
            encryptedUtxos: new Uint8Array(256).fill(1),
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
    });
    // should pass because no non-zero input utxo is provided
    (0, mocha_1.it)("No in utxo test invalid root", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParams,
        });
        await tx.compile();
        tx.proofInput.root = new anchor_1.BN("123").toString();
        await tx.getProof();
    });
    (0, mocha_1.it)("With in utxo test invalid root", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.root = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid tx integrity hash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.txIntegrityHash = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("No in utxo test invalid publicMintPubkey", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParams,
        });
        await tx.compile();
        tx.proofInput.publicMintPubkey = (0, src_1.hashAndTruncateToCircuit)(web3_js_1.Keypair.generate().publicKey.toBytes());
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid publicMintPubkey", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.publicMintPubkey = (0, src_1.hashAndTruncateToCircuit)(web3_js_1.Keypair.generate().publicKey.toBytes());
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    // should succeed because no public spl amount is provided thus mint is not checked
    (0, mocha_1.it)("No public spl amount test invalid publicMintPubkey", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsSol,
        });
        await tx.compile();
        tx.proofInput.publicMintPubkey = (0, src_1.hashAndTruncateToCircuit)(web3_js_1.Keypair.generate().publicKey.toBytes());
        await tx.getProof();
    });
    (0, mocha_1.it)("With in utxo test invalid merkle proof path elements", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inPathElements[0] =
            tx.provider.solMerkleTree?.merkleTree.path(1).pathElements;
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid merkle proof path index", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inPathIndices[0] = 1;
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inPrivateKey", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inPrivateKey[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid publicAmountSpl", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.publicAmountSpl = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid publicAmountSol", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.publicAmountSol = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid publicAmountSpl", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsSol,
        });
        await tx.compile();
        tx.proofInput.publicAmountSpl = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outputCommitment", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        console.log();
        tx.proofInput.outputCommitment[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inAmount", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inAmount[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outAmount", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outAmount[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inBlinding", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inBlinding[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outBlinding", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outBlinding[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outPubkey", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outPubkey[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid assetPubkeys", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        for (let i = 0; i < 3; i++) {
            tx.proofInput.assetPubkeys[i] = (0, src_1.hashAndTruncateToCircuit)(web3_js_1.Keypair.generate().publicKey.toBytes());
            await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
        }
    });
    // this fails because the system verifier does not allow
    (0, mocha_1.it)("With in utxo test invalid inAppDataHash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsApp,
            appParams: { mock: "1231", verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO },
        });
        await tx.compile();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    // this works because the system verifier does not check output utxos other than commit hashes being well-formed and the sum
    (0, mocha_1.it)("With out utxo test inAppDataHash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsOutApp,
        });
        await tx.compile();
        await tx.getProof();
    });
    // this fails because it's inconsistent with the utxo
    (0, mocha_1.it)("With in utxo test invalid outAppDataHash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outAppDataHash[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid pooltype", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsPoolType,
        });
        await tx.compile();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With out utxo test invalid pooltype", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsPoolTypeOut,
        });
        await tx.compile();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inPoolType", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inPoolType[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outPoolType", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outPoolType[0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inIndices", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.inIndices[0][0][0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid inIndices", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
        tx.proofInput.inIndices[1][1][1] = "1";
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outIndices", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        tx.proofInput.outIndices[0][0][0] = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("With in utxo test invalid outIndices", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsWithdrawal,
        });
        await tx.compile();
        chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
        tx.proofInput.outIndices[1][1][1] = "1";
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
});
// TODO: check more specific errors in tests
describe("App system circuit tests", () => {
    let lightProvider;
    before(async () => {
        lightProvider = await src_1.Provider.loadMock();
        poseidon = await buildPoseidonOpt();
        account = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        await account.getEddsaPublicKey();
        let depositAmount = 20000;
        let depositFeeAmount = 10000;
        deposit_utxo1 = new src_1.Utxo({
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: account,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        mockPubkey = web3_js_1.Keypair.generate().publicKey;
        let relayerPubkey = web3_js_1.Keypair.generate().publicKey;
        lightProvider = await src_1.Provider.loadMock();
        txParams = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.SHIELD,
            poseidon,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_TWO,
        });
        relayer = new src_1.Relayer(relayerPubkey, mockPubkey, new anchor_1.BN(5000));
        txParamsApp = new src_1.TransactionParameters({
            inputUtxos: [
                new src_1.Utxo({
                    poseidon,
                    appData,
                    appDataIdl: tmp_test_psp_1.IDL,
                    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
                }),
            ],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet.publicKey,
            action: src_1.Action.UNSHIELD,
            poseidon,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_TWO,
        });
    });
    (0, mocha_1.it)("No in utxo test invalid transactionHash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParams,
            appParams: { mock: "123", verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO },
        });
        await tx.compile();
        tx.proofInput.transactionHash = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
    (0, mocha_1.it)("No in utxo test invalid transactionHash", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: txParamsApp,
            appParams: { mock: "123", verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO },
        });
        await tx.compile();
        tx.proofInput.publicAppVerifier = new anchor_1.BN("123").toString();
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_GENERATION_FAILED);
    });
});
//# sourceMappingURL=circuits.test.js.map