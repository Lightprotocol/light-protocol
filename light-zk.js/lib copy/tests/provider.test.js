"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
//@ts-nocheck
const chai_1 = require("chai");
let circomlibjs = require("circomlibjs");
const web3_js_1 = require("@solana/web3.js");
const mocha_1 = require("mocha");
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);
const src_1 = require("../src");
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
describe("Test Provider Functional", () => {
    let poseidon;
    before(async () => {
        poseidon = await circomlibjs.buildPoseidonOpt();
    });
    (0, mocha_1.it)("Mock Provider", async () => {
        const lightProviderMock = await src_1.Provider.loadMock();
        chai_1.assert.equal(lightProviderMock.wallet.isNodeWallet, true);
        chai_1.assert.equal(lightProviderMock.wallet?.publicKey.toBase58(), src_1.ADMIN_AUTH_KEYPAIR.publicKey.toBase58());
        chai_1.assert.equal(lightProviderMock.url, "mock");
        (0, chai_1.assert)(lightProviderMock.poseidon);
        (0, chai_1.assert)(lightProviderMock.lookUpTables.versionedTransactionLookupTable);
        chai_1.assert.equal(lightProviderMock.solMerkleTree?.pubkey.toBase58(), src_1.MerkleTreeConfig.getTransactionMerkleTreePda().toBase58());
        chai_1.assert.equal(lightProviderMock.solMerkleTree?.merkleTree.levels, 18);
        chai_1.assert.equal(lightProviderMock.solMerkleTree?.merkleTree.zeroElement, src_1.DEFAULT_ZERO);
        const additionalMint = web3_js_1.Keypair.generate().publicKey;
        chai_1.assert.equal(lightProviderMock.lookUpTables.assetLookupTable[0], web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(lightProviderMock.lookUpTables.assetLookupTable[1], src_1.MINT.toBase58());
        chai_1.assert.equal(lightProviderMock.lookUpTables.verifierProgramLookupTable[0], web3_js_1.SystemProgram.programId.toBase58());
        chai_1.assert.equal(lightProviderMock.lookUpTables.verifierProgramLookupTable[1], "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
        lightProviderMock.addAssetPublickeyToLookUpTable(additionalMint);
        chai_1.assert.equal(lightProviderMock.lookUpTables.assetLookupTable[2], additionalMint.toBase58());
        lightProviderMock.addVerifierProgramPublickeyToLookUpTable(additionalMint);
        chai_1.assert.equal(lightProviderMock.lookUpTables.verifierProgramLookupTable[2], additionalMint.toBase58());
    });
    (0, mocha_1.it)("KEYPAIR_UNDEFINED Provider", async () => {
        await chai.assert.isRejected(
        // @ts-ignore
        src_1.Provider.init({}), src_1.ProviderErrorCode.KEYPAIR_UNDEFINED);
    });
    (0, mocha_1.it)("WALLET_UNDEFINED", async () => {
        (0, chai_1.expect)(() => {
            // @ts-ignore
            new src_1.Provider({});
        })
            .to.throw(src_1.ProviderError)
            .includes({
            code: src_1.ProviderErrorCode.WALLET_UNDEFINED,
            functionName: "constructor",
        });
    });
});
//# sourceMappingURL=provider.test.js.map