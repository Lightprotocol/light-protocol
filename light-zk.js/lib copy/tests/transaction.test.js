"use strict";
//@ts-nocheck
Object.defineProperty(exports, "__esModule", { value: true });
const chai_1 = require("chai");
let circomlibjs = require("circomlibjs");
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const mocha_1 = require("mocha");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const src_1 = require("../src");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("Transaction Error Tests", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey2 = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
    let poseidon, lightProvider, deposit_utxo1, relayer, keypair, params;
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
        params = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
    });
    (0, mocha_1.it)("Constructor PROVIDER_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore:
            new src_1.Transaction({
                params,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.PROVIDER_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("Constructor POSEIDON_HASHER_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Transaction({
                // @ts-ignore:
                provider: {},
                params,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.POSEIDON_HASHER_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("Constructor SOL_MERKLE_TREE_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Transaction({
                // @ts-ignore:
                provider: { poseidon },
                params,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("Constructor WALLET_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Transaction({
                // @ts-ignore:
                provider: { poseidon, solMerkleTree: {} },
                params,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.WALLET_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("Constructor WALLET_RELAYER_INCONSISTENT", async () => {
        const params1 = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            senderSpl: mockPubkey,
            senderSol: mockPubkey,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        (0, chai_1.expect)(() => {
            new src_1.Transaction({
                provider: lightProvider,
                params: params1,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.WALLET_RELAYER_INCONSISTENT,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("Constructor TX_PARAMETERS_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore:
            new src_1.Transaction({
                provider: lightProvider,
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("getProof VERIFIER_IDL_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            new src_1.Transaction({
                provider: lightProvider,
                // @ts-ignore
                params: {},
            });
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.VERIFIER_IDL_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("getProofInternal PROOF_INPUT_UNDEFINED", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params,
        });
        await chai.assert.isRejected(tx.getProof(), src_1.TransactionErrorCode.PROOF_INPUT_UNDEFINED);
    });
    (0, mocha_1.it)("getAppProof APP_PARAMETERS_UNDEFINED", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params,
        });
        await chai.assert.isRejected(tx.getAppProof(), src_1.TransactionErrorCode.APP_PARAMETERS_UNDEFINED);
    });
    (0, mocha_1.it)("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
        let tx = new src_1.Transaction({
            provider: {
                // @ts-ignore
                solMerkleTree: {},
                poseidon,
                wallet: lightProvider.wallet,
            },
            params,
        });
        await chai.assert.isRejected(tx.getRootIndex(), src_1.SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED);
    });
    (0, mocha_1.it)("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
        let tx = new src_1.Transaction({
            provider: {
                // @ts-ignore
                solMerkleTree: {},
                poseidon,
                wallet: lightProvider.wallet,
            },
            params,
        });
        await chai.assert.isRejected(tx.getRootIndex(), src_1.SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED);
    });
    (0, mocha_1.it)("getRootIndex MERKLE_TREE_UNDEFINED", async () => {
        let tx = new src_1.Transaction({
            // @ts-ignore
            provider: lightProvider,
            params,
        });
        // @ts-ignore
        tx.params.assetPubkeysCircuit = undefined;
        (0, chai_1.expect)(() => {
            tx.getIndices(params.inputUtxos);
        })
            .throw(src_1.TransactionError)
            .includes({
            code: src_1.TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
            functionName: "getIndices",
        });
    });
});
describe("Transaction Functional Tests", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey2 = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
    let poseidon, lightProvider, deposit_utxo1, relayer, keypair, paramsDeposit, paramsWithdrawal;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey3, mockPubkey, new anchor_1.BN(5000));
        keypair = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        lightProvider = await src_1.Provider.loadMock();
        deposit_utxo1 = new src_1.Utxo({
            index: 0,
            poseidon: poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
            account: keypair,
            blinding: new anchor_1.BN(new Array(31).fill(1)),
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        paramsDeposit = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            senderSpl: mockPubkey,
            senderSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        lightProvider.solMerkleTree.merkleTree = new src_1.MerkleTree(18, poseidon, [
            deposit_utxo1.getCommitment(poseidon),
        ]);
        chai_1.assert.equal(lightProvider.solMerkleTree?.merkleTree.indexOf(deposit_utxo1.getCommitment(poseidon)), 0);
        paramsWithdrawal = new src_1.TransactionParameters({
            inputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            recipientSpl: mockPubkey,
            recipientSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.UNSHIELD,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
    });
    (0, mocha_1.it)("Functional ", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        await tx.compileAndProve();
    });
    (0, mocha_1.it)("Functional storage ", async () => {
        const paramsDepositStorage = new src_1.TransactionParameters({
            message: Buffer.alloc(928).fill(1),
            inputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey2,
            transactionMerkleTreePubkey: mockPubkey2,
            poseidon,
            recipientSpl: mockPubkey,
            recipientSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.UNSHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_STORAGE,
            relayer,
        });
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDepositStorage,
        });
        await tx.compileAndProve();
        await tx.getInstructions(tx.params);
    });
    (0, mocha_1.it)("getMint ", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        let mint = tx.getMint();
        chai_1.assert.equal(mint.toString(), (0, src_1.hashAndTruncateToCircuit)(src_1.MINT.toBuffer()).toString());
        chai_1.assert.notEqual(mint.toString(), src_1.MINT.toString());
    });
    (0, mocha_1.it)("getRootIndex Provider Undefined", async () => {
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsDeposit,
        });
        await tx.getRootIndex();
        chai_1.assert.equal(tx.transactionInputs.rootIndex?.toNumber(), 0);
    });
    (0, mocha_1.it)("getIndices", async () => {
        const poseidon = await circomlibjs.buildPoseidonOpt();
        let mockPubkey = web3_js_1.Keypair.generate().publicKey;
        let lightProvider = await src_1.Provider.loadMock();
        let deposit_utxo1 = new src_1.Utxo({
            poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_1, src_1.BN_2],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        const relayer = new src_1.Relayer(mockPubkey, mockPubkey, new anchor_1.BN(5000));
        let params = new src_1.TransactionParameters({
            inputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            recipientSpl: mockPubkey,
            recipientSol: mockPubkey,
            poseidon,
            action: src_1.Action.UNSHIELD,
            relayer,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params,
        });
        const indices1 = tx.getIndices([deposit_utxo1]);
        chai_1.assert.equal(indices1[0][0][0], "1");
        chai_1.assert.equal(indices1[0][0][1], "0");
        chai_1.assert.equal(indices1[0][0][2], "0");
        chai_1.assert.equal(indices1[0][1][0], "0");
        chai_1.assert.equal(indices1[0][1][1], "1");
        chai_1.assert.equal(indices1[0][1][2], "0");
        const indices2 = tx.getIndices([deposit_utxo1, deposit_utxo1]);
        chai_1.assert.equal(indices2[0][0][0], "1");
        chai_1.assert.equal(indices2[0][0][1], "0");
        chai_1.assert.equal(indices2[0][0][2], "0");
        chai_1.assert.equal(indices2[0][1][0], "0");
        chai_1.assert.equal(indices2[0][1][1], "1");
        chai_1.assert.equal(indices2[0][1][2], "0");
        let deposit_utxo2 = new src_1.Utxo({
            poseidon,
            assets: [src_1.FEE_ASSET],
            amounts: [src_1.BN_1],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        const indices3 = tx.getIndices([deposit_utxo2]);
        chai_1.assert.equal(indices3[0][0][0], "1");
        chai_1.assert.equal(indices3[0][0][1], "0");
        chai_1.assert.equal(indices3[0][0][2], "0");
        chai_1.assert.equal(indices3[0][1][0], "0");
        chai_1.assert.equal(indices3[0][1][1], "0");
        chai_1.assert.equal(indices3[0][1][2], "0");
        let deposit_utxo3 = new src_1.Utxo({
            poseidon,
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        const indices4 = tx.getIndices([deposit_utxo3]);
        chai_1.assert.equal(indices4[0][0][0], "0");
        chai_1.assert.equal(indices4[0][0][1], "0");
        chai_1.assert.equal(indices4[0][0][2], "0");
        chai_1.assert.equal(indices4[0][1][0], "0");
        chai_1.assert.equal(indices4[0][1][1], "0");
        chai_1.assert.equal(indices4[0][1][2], "0");
        let deposit_utxo4 = new src_1.Utxo({
            poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_0, src_1.BN_2],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        const indices5 = tx.getIndices([deposit_utxo4]);
        chai_1.assert.equal(indices5[0][0][0], "1");
        chai_1.assert.equal(indices5[0][0][1], "0");
        chai_1.assert.equal(indices5[0][0][2], "0");
        chai_1.assert.equal(indices5[0][1][0], "0");
        chai_1.assert.equal(indices5[0][1][1], "1");
        chai_1.assert.equal(indices5[0][1][2], "0");
        const indices6 = tx.getIndices([deposit_utxo3, deposit_utxo4]);
        chai_1.assert.equal(indices6[0][0][0], "0");
        chai_1.assert.equal(indices6[0][0][1], "0");
        chai_1.assert.equal(indices6[0][0][2], "0");
        chai_1.assert.equal(indices6[0][1][0], "0");
        chai_1.assert.equal(indices6[0][1][1], "0");
        chai_1.assert.equal(indices6[0][1][2], "0");
        chai_1.assert.equal(indices6[1][0][0], "1");
        chai_1.assert.equal(indices6[1][0][1], "0");
        chai_1.assert.equal(indices6[1][0][2], "0");
        chai_1.assert.equal(indices6[1][1][0], "0");
        chai_1.assert.equal(indices6[1][1][1], "1");
        chai_1.assert.equal(indices6[1][1][2], "0");
        let deposit_utxo5 = new src_1.Utxo({
            poseidon,
            assets: [src_1.FEE_ASSET, src_1.MINT],
            amounts: [src_1.BN_2, src_1.BN_0],
            assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
            verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable,
        });
        const indices7 = tx.getIndices([deposit_utxo5]);
        chai_1.assert.equal(indices7[0][0][0], "1");
        chai_1.assert.equal(indices7[0][0][1], "0");
        chai_1.assert.equal(indices7[0][0][2], "0");
        chai_1.assert.equal(indices7[0][1][0], "0");
        chai_1.assert.equal(indices7[0][1][1], "1");
        chai_1.assert.equal(indices7[0][1][2], "0");
    });
    (0, mocha_1.it)("getConnectingHash", async () => {
        const relayerConst = new src_1.Relayer(src_1.AUTHORITY, src_1.AUTHORITY, new anchor_1.BN(5000));
        const paramsStaticEncryptedUtxos = new src_1.TransactionParameters({
            inputUtxos: [deposit_utxo1, deposit_utxo1],
            outputUtxos: [deposit_utxo1, deposit_utxo1],
            eventMerkleTreePubkey: src_1.AUTHORITY,
            transactionMerkleTreePubkey: src_1.AUTHORITY,
            poseidon,
            recipientSpl: src_1.AUTHORITY,
            recipientSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.UNSHIELD,
            relayer: relayerConst,
            encryptedUtxos: new Uint8Array(256).fill(1),
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        let txIntegrityHash = await paramsStaticEncryptedUtxos.getTxIntegrityHash(poseidon);
        chai_1.assert.equal(txIntegrityHash.toString(), "6150353308703750134875659224593639995108994571023605893130935914916250029450");
        chai_1.assert.equal(src_1.Transaction.getTransactionHash(paramsStaticEncryptedUtxos, poseidon).toString(), "5933194464001103981860458884656917415381806542379509455129642519383560866951");
    });
    (0, mocha_1.it)("getMerkleProof", async () => {
        let merkleProofsDeposit = src_1.Transaction.getMerkleProofs(lightProvider, paramsDeposit.inputUtxos);
        chai_1.assert.equal(merkleProofsDeposit.inputMerklePathIndices.toString(), new Array(2).fill("0").toString());
        chai_1.assert.equal(merkleProofsDeposit.inputMerklePathElements[0].toString(), new Array(18).fill("0").toString());
        chai_1.assert.equal(merkleProofsDeposit.inputMerklePathElements[1].toString(), new Array(18).fill("0").toString());
        let merkleProofsWithdrawal = src_1.Transaction.getMerkleProofs(lightProvider, paramsWithdrawal.inputUtxos);
        chai_1.assert.equal(merkleProofsWithdrawal.inputMerklePathIndices.toString(), new Array(2).fill("0").toString());
        const constElements = [
            "14522046728041339886521211779101644712859239303505368468566383402165481390632",
            "12399300409582020702502593817695692114365413884629119646752088755594619792099",
            "8395588225108361090185968542078819429341401311717556516132539162074718138649",
            "4057071915828907980454096850543815456027107468656377022048087951790606859731",
            "3743829818366380567407337724304774110038336483209304727156632173911629434824",
            "3362607757998999405075010522526038738464692355542244039606578632265293250219",
            "20015677184605935901566129770286979413240288709932102066659093803039610261051",
            "10225829025262222227965488453946459886073285580405166440845039886823254154094",
            "5686141661288164258066217031114275192545956158151639326748108608664284882706",
            "13358779464535584487091704300380764321480804571869571342660527049603988848871",
            "20788849673815300643597200320095485951460468959391698802255261673230371848899",
            "18755746780925592439082197927133359790105305834996978755923950077317381403267",
            "10861549147121384785495888967464291400837754556942768811917754795517438910238",
            "7537538922575546318235739307792157434585071385790082150452199061048979169447",
            "19170203992070410766412159884086833170469632707946611516547317398966021022253",
            "9623414539891033920851862231973763647444234218922568879041788217598068601671",
            "3060533073600086539557684568063736193011911125938770961176821146879145827363",
            "138878455357257924790066769656582592677416924479878379980482552822708744793",
        ];
        chai_1.assert.equal(merkleProofsWithdrawal.inputMerklePathElements[0].toString(), constElements.toString());
        chai_1.assert.equal(merkleProofsWithdrawal.inputMerklePathElements[1].toString(), new Array(18).fill("0").toString());
    });
    (0, mocha_1.it)("getPdaAddresses", async () => {
        const relayerConst = new src_1.Relayer(src_1.AUTHORITY, src_1.AUTHORITY, new anchor_1.BN(5000));
        const paramsStaticEncryptedUtxos = new src_1.TransactionParameters({
            inputUtxos: [deposit_utxo1, deposit_utxo1],
            outputUtxos: [deposit_utxo1, deposit_utxo1],
            eventMerkleTreePubkey: src_1.AUTHORITY,
            transactionMerkleTreePubkey: src_1.AUTHORITY,
            poseidon,
            recipientSpl: src_1.AUTHORITY,
            recipientSol: lightProvider.wallet?.publicKey,
            action: src_1.Action.UNSHIELD,
            relayer: relayerConst,
            encryptedUtxos: new Uint8Array(256).fill(1),
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        let tx = new src_1.Transaction({
            provider: lightProvider,
            params: paramsStaticEncryptedUtxos,
        });
        // @ts-ignore
        tx.transactionInputs.publicInputs = { leaves: [], nullifiers: [] };
        tx.transactionInputs.publicInputs.outputCommitment = [
            new Array(32).fill(1),
            new Array(32).fill(1),
        ];
        tx.transactionInputs.publicInputs.inputNullifier = [
            new Array(32).fill(1),
            new Array(32).fill(1),
        ];
        tx.getPdaAddresses();
        const refNullfiers = [
            "A3rueqakAhxjJVUrygVZdpd3wUNUHiGuKy2M7zR7uHDh",
            "A3rueqakAhxjJVUrygVZdpd3wUNUHiGuKy2M7zR7uHDh",
        ];
        const refLeaves = ["6UuSTaJpEemGVuPkmtTiNe7VndXXenWCDU49aTkGSQqY"];
        for (let i = 0; i < 2; i++) {
            chai_1.assert.equal(tx.remainingAccounts?.nullifierPdaPubkeys[i].pubkey.toBase58(), refNullfiers[i]);
        }
        chai_1.assert.equal(tx.remainingAccounts?.leavesPdaPubkeys[0].pubkey.toBase58(), refLeaves[0]);
        chai_1.assert.equal(tx.params.accounts.verifierState.toBase58(), "5XAf8s2hi4fx3QK8fm6dgkfXLE23Hy9k1Qo3ew6QqdGP");
    });
    (0, mocha_1.it)("APP_PARAMETERS_UNDEFINED", async () => {
        const params = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: mockPubkey,
            poseidon,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_TWO,
        });
        (0, chai_1.expect)(() => {
            let tx = new src_1.Transaction({
                provider: lightProvider,
                params,
            });
        })
            .to.throw(src_1.TransactionError)
            .to.include({
            code: src_1.TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
            functionName: "constructor",
        });
    });
    (0, mocha_1.it)("INVALID_VERIFIER_SELECTED", async () => {
        const params = new src_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            eventMerkleTreePubkey: mockPubkey,
            transactionMerkleTreePubkey: mockPubkey,
            senderSpl: mockPubkey,
            senderSol: mockPubkey,
            poseidon,
            action: src_1.Action.SHIELD,
            verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO,
        });
        (0, chai_1.expect)(() => {
            let tx = new src_1.Transaction({
                provider: lightProvider,
                params,
                appParams: { mock: "1231", verifierIdl: src_1.IDL_VERIFIER_PROGRAM_ZERO },
            });
        })
            .to.throw(src_1.TransactionError)
            .to.include({
            code: src_1.TransactionErrorCode.INVALID_VERIFIER_SELECTED,
            functionName: "constructor",
        });
    });
});
//# sourceMappingURL=transaction.test.js.map