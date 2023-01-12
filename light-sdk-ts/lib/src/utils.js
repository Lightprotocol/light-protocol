"use strict";
var __awaiter =
  (this && this.__awaiter) ||
  function (thisArg, _arguments, P, generator) {
    function adopt(value) {
      return value instanceof P
        ? value
        : new P(function (resolve) {
            resolve(value);
          });
    }
    return new (P || (P = Promise))(function (resolve, reject) {
      function fulfilled(value) {
        try {
          step(generator.next(value));
        } catch (e) {
          reject(e);
        }
      }
      function rejected(value) {
        try {
          step(generator["throw"](value));
        } catch (e) {
          reject(e);
        }
      }
      function step(result) {
        result.done
          ? resolve(result.value)
          : adopt(result.value).then(fulfilled, rejected);
      }
      step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.fetchAssetByIdLookUp =
  exports.getAssetLookUpId =
  exports.hashAndTruncateToCircuit =
    void 0;
const anchor_1 = require("@project-serum/anchor");
const ethers_1 = require("ethers");
const constants_1 = require("./constants");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const web3_js_1 = require("@solana/web3.js");
const merkleTree_1 = require("./merkleTree");
// TODO: get rid of ether dep
function hashAndTruncateToCircuit(data) {
  return new anchor_1.BN(
    leInt2Buff(
      unstringifyBigInts(ethers_1.ethers.utils.keccak256(data).toString()),
      32
    )
      .reverse()
      .slice(1, 32),
    undefined,
    "be"
  );
}
exports.hashAndTruncateToCircuit = hashAndTruncateToCircuit;
// TODO: add pooltype
function getAssetLookUpId({ connection, asset }) {
  return __awaiter(this, void 0, void 0, function* () {
    let poolType = new Uint8Array(32).fill(0);
    let mtConf = new merkleTree_1.MerkleTreeConfig({
      connection,
      merkleTreePubkey: constants_1.MERKLE_TREE_KEY,
    });
    let pubkey = yield mtConf.getSplPoolPda(poolType, asset);
    let registeredAssets =
      yield constants_1.merkleTreeProgram.account.registeredAssetPool.fetch(
        pubkey.pda
      );
    return registeredAssets.index;
  });
}
exports.getAssetLookUpId = getAssetLookUpId;
function fetchAssetByIdLookUp({ assetIndex }) {
  // TODO: find smarter way to do this maybe query from account
  console.log("here ", assetIndex);
  let poolType = new Uint8Array(32).fill(0);
  if (assetIndex.toString() == "0") {
    return constants_1.MINT;
  } else if (assetIndex.toString() == "1") {
    return web3_js_1.SystemProgram.programId;
  } else {
    throw `no entry for index ${assetIndex}`;
  }
  // let registeredAssets = await merkleTreeProgram.account.registeredAssetPool.all();
  // // console.log("registeredAssets ", registeredAssets.publictoBase58());
  // return registeredAssets.index;
  // let x
  // registeredAssets.map((a)=> {
  //   console.log("a.account.assetPoolPubkey.toBase58() ", a.account.index.toString());
  //   console.log("asset.toBase58() ", assetIndex.toString());
  //   if(a.account.index.toString() == assetIndex.toString()) {
  //     console.log("returned ", a.account.pubkey);
  //     console.log("returned ", a);
  //     x = a.account.assetPoolPubkey;
  // }});
  // return x;
}
exports.fetchAssetByIdLookUp = fetchAssetByIdLookUp;
