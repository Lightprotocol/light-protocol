"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.fetchAssetByIdLookUp = exports.getAssetIndex = exports.assetLookupTable = exports.getAssetLookUpId = exports.hashAndTruncateToCircuit = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const constants_1 = require("./constants");
const web3_js_1 = require("@solana/web3.js");
const merkleTree_1 = require("./merkleTree");
const constants_system_verifier_1 = require("./test-utils/constants_system_verifier");
const { keccak_256 } = require("@noble/hashes/sha3");
function hashAndTruncateToCircuit(data) {
    return new anchor_1.BN(keccak_256
        .create({ dkLen: 32 })
        .update(Buffer.from(data))
        .digest()
        .slice(1, 32), undefined, "be");
}
exports.hashAndTruncateToCircuit = hashAndTruncateToCircuit;
// TODO: add pooltype
function getAssetLookUpId({ connection, asset, }) {
    return __awaiter(this, void 0, void 0, function* () {
        let poolType = new Uint8Array(32).fill(0);
        let mtConf = new merkleTree_1.MerkleTreeConfig({
            connection,
            merkleTreePubkey: constants_1.MERKLE_TREE_KEY,
        });
        let pubkey = yield mtConf.getSplPoolPda(poolType, asset);
        let registeredAssets = yield mtConf.merkleTreeProgram.account.registeredAssetPool.fetch(pubkey.pda);
        return registeredAssets.index;
    });
}
exports.getAssetLookUpId = getAssetLookUpId;
// TODO: fetch from chain
// TODO: separate testing variables from prod env
exports.assetLookupTable = [web3_js_1.SystemProgram.programId, constants_system_verifier_1.MINT];
function getAssetIndex(assetPubkey) {
    return new anchor_1.BN(exports.assetLookupTable.indexOf(assetPubkey));
}
exports.getAssetIndex = getAssetIndex;
function fetchAssetByIdLookUp(assetIndex) {
    return exports.assetLookupTable[assetIndex.toNumber()];
}
exports.fetchAssetByIdLookUp = fetchAssetByIdLookUp;
