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
const src_1 = require("../src");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("Utxo Functional", () => {
    let seed32 = bytes_1.bs58.encode(new Uint8Array(32).fill(1));
    let depositAmount = 20000;
    let depositFeeAmount = 10000;
    let mockPubkey = web3_js_1.Keypair.generate().publicKey;
    let mockPubkey3 = web3_js_1.Keypair.generate().publicKey;
    let poseidon, lightProvider, deposit_utxo1, relayer, keypair;
    before(async () => {
        poseidon = await buildPoseidonOpt();
        // TODO: make fee mandatory
        relayer = new src_1.Relayer(mockPubkey3, mockPubkey, new anchor_1.BN(5000));
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
    (0, mocha_1.it)("Test Balance moveToSpentUtxos", async () => {
        let balance = {
            tokenBalances: new Map([
                [web3_js_1.SystemProgram.programId.toBase58(), src_1.TokenUtxoBalance.initSol()],
            ]),
            totalSolBalance: src_1.BN_0,
            programBalances: new Map(),
            nftBalances: new Map(),
        };
        let tokenBalanceUsdc = new src_1.TokenUtxoBalance(src_1.TOKEN_REGISTRY.get("USDC"));
        balance.tokenBalances.set(tokenBalanceUsdc.tokenData.mint.toBase58(), tokenBalanceUsdc);
        balance.tokenBalances
            .get(src_1.MINT.toBase58())
            ?.addUtxo(deposit_utxo1.getCommitment(poseidon), deposit_utxo1, "utxos");
        src_1.Utxo.equal(poseidon, await balance.tokenBalances
            .get(src_1.MINT.toBase58())
            ?.utxos.get(deposit_utxo1.getCommitment(poseidon)), await deposit_utxo1);
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.totalBalanceSol.toString(), deposit_utxo1.amounts[0].toString());
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.totalBalanceSpl.toString(), deposit_utxo1.amounts[1].toString());
        chai_1.assert.equal(balance.tokenBalances.get(web3_js_1.SystemProgram.programId.toBase58())?.spentUtxos
            .size, 0);
        balance.tokenBalances
            .get(src_1.MINT.toBase58())
            ?.moveToSpentUtxos(deposit_utxo1.getCommitment(poseidon));
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.totalBalanceSol.toString(), "0");
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.totalBalanceSpl.toString(), "0");
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.spentUtxos.size, 1);
        chai_1.assert.equal(balance.tokenBalances.get(src_1.MINT.toBase58())?.utxos.size, 0);
        src_1.Utxo.equal(poseidon, await balance.tokenBalances
            .get(src_1.MINT.toBase58())
            ?.spentUtxos.get(deposit_utxo1.getCommitment(poseidon)), await deposit_utxo1);
    });
    // this test is a mock of the syncState function
    // TODO: add a direct test
    (0, mocha_1.it)("Test Decrypt Balance 2 and 4 utxos", async () => {
        const provider = await src_1.Provider.loadMock();
        let verifierProgramLookupTable = provider.lookUpTables.verifierProgramLookupTable;
        let assetLookupTable = provider.lookUpTables.assetLookupTable;
        const account = new src_1.Account({ poseidon: poseidon, seed: seed32 });
        for (let j = 2; j < 4; j += 2) {
            let utxos = [];
            let encryptedUtxos = [];
            for (let index = 0; index < j; index++) {
                const depositAmount = index;
                const depositFeeAmount = index;
                const utxo = new src_1.Utxo({
                    poseidon: poseidon,
                    assets: [src_1.FEE_ASSET, src_1.MINT],
                    amounts: [new anchor_1.BN(depositFeeAmount), new anchor_1.BN(depositAmount)],
                    account: account,
                    index: index,
                    assetLookupTable: provider.lookUpTables.assetLookupTable,
                    verifierProgramLookupTable: provider.lookUpTables.verifierProgramLookupTable,
                    blinding: new anchor_1.BN(1),
                });
                utxos.push(utxo);
                encryptedUtxos = [
                    ...encryptedUtxos,
                    ...(await utxo.encrypt(poseidon, src_1.MerkleTreeConfig.getTransactionMerkleTreePda(), true)),
                ];
            }
            let indexedTransactions = [
                {
                    leaves: utxos.map((utxo) => new anchor_1.BN(utxo.getCommitment(poseidon)).toBuffer("le", 32)),
                    firstLeafIndex: "0",
                    encryptedUtxos,
                },
            ];
            let decryptedUtxos = new Array();
            for (const trx of indexedTransactions) {
                let leftLeafIndex = new anchor_1.BN(trx.firstLeafIndex).toNumber();
                for (let index = 0; index < trx.leaves.length; index += 2) {
                    const leafLeft = trx.leaves[index];
                    const leafRight = trx.leaves[index + 1];
                    let decryptedUtxo = await src_1.Utxo.decrypt({
                        poseidon,
                        encBytes: Buffer.from(trx.encryptedUtxos.slice((index / 2) * 240, (index / 2) * 240 + src_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH)),
                        account,
                        index: leftLeafIndex + index,
                        commitment: leafLeft,
                        aes: true,
                        merkleTreePdaPublicKey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
                        verifierProgramLookupTable,
                        assetLookupTable,
                    });
                    decryptedUtxos.push(decryptedUtxo);
                    decryptedUtxo = await src_1.Utxo.decrypt({
                        poseidon,
                        encBytes: Buffer.from(trx.encryptedUtxos.slice((index / 2) * 240 + 120, (index / 2) * 240 +
                            src_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
                            120)),
                        account,
                        index: leftLeafIndex + index + 1,
                        commitment: leafRight,
                        aes: true,
                        merkleTreePdaPublicKey: src_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
                        verifierProgramLookupTable,
                        assetLookupTable,
                    });
                    decryptedUtxos.push(decryptedUtxo);
                }
            }
            utxos.map((utxo, index) => {
                src_1.Utxo.equal(poseidon, utxo, decryptedUtxos[index]);
            });
        }
    });
});
//# sourceMappingURL=balance.test.js.map