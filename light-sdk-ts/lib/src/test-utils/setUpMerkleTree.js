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
exports.setUpMerkleTree = void 0;
const chai_1 = require("chai");
const constants_1 = require("../constants");
const merkleTreeConfig_1 = require("../merkleTree/merkleTreeConfig");
function setUpMerkleTree(provider) {
  return __awaiter(this, void 0, void 0, function* () {
    var merkleTreeAccountInfoInit = yield provider.connection.getAccountInfo(
      constants_1.MERKLE_TREE_KEY,
    );
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    console.log("MERKLE_TREE_KEY ", constants_1.MERKLE_TREE_KEY);
    console.log("ADMIN_AUTH_KEYPAIR ", constants_1.ADMIN_AUTH_KEYPAIR);
    if (merkleTreeAccountInfoInit == null) {
      let merkleTreeConfig = new merkleTreeConfig_1.MerkleTreeConfig({
        merkleTreePubkey: constants_1.MERKLE_TREE_KEY,
        payer: constants_1.ADMIN_AUTH_KEYPAIR,
        connection: provider.connection,
      });
      console.log("Initing MERKLE_TREE_AUTHORITY_PDA");
      try {
        const ix = yield merkleTreeConfig.initMerkleTreeAuthority();
        console.log("initMerkleTreeAuthority success, ", ix);
        // assert(await provider.connection.getTransaction(ix, {commitment:"confirmed"}) != null, "init failed");
      } catch (e) {
        console.log(e);
      }
      console.log("AUTHORITY: ", constants_1.AUTHORITY);
      console.log("AUTHORITY: ", Array.from(constants_1.AUTHORITY.toBytes()));
      console.log(
        "verifierProgramZero.programId: ",
        Array.from(constants_1.verifierProgramZero.programId.toBytes()),
      );
      console.log("MERKLE_TREE_KEY: ", constants_1.MERKLE_TREE_KEY.toBase58());
      console.log(
        "MERKLE_TREE_KEY: ",
        Array.from(constants_1.MERKLE_TREE_KEY.toBytes()),
      );
      // console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
      // console.log("MERKLE_TREE_PDA_TOKEN: ", Array.from(MERKLE_TREE_PDA_TOKEN.toBytes()))
      try {
        const ix = yield merkleTreeConfig.initializeNewMerkleTree();
        (0, chai_1.assert)(
          (yield provider.connection.getTransaction(ix, {
            commitment: "confirmed",
          })) != null,
          "init failed",
        );
      } catch (e) {
        console.log(e);
      }
      console.log("Registering Verifier");
      try {
        yield merkleTreeConfig.registerVerifier(
          constants_1.verifierProgramZero.programId,
        );
        console.log("Registering Verifier Zero success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerVerifier(
          constants_1.verifierProgramOne.programId,
        );
        console.log("Registering Verifier One success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerVerifier(
          constants_1.verifierProgramTwo.programId,
        );
        console.log("Registering Verifier One success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerPoolType(constants_1.POOL_TYPE);
        console.log("Registering pool_type success");
      } catch (e) {
        console.log(e);
      }
      console.log("MINT: ", constants_1.MINT.toBase58());
      console.log(
        "POOL_TYPE_PDA: ",
        constants_1.REGISTERED_POOL_PDA_SPL.toBase58(),
      );
      try {
        yield merkleTreeConfig.registerSplPool(
          constants_1.POOL_TYPE,
          constants_1.MINT,
        );
        console.log("Registering spl pool success");
      } catch (e) {
        console.log(e);
      }
      console.log(
        "REGISTERED_POOL_PDA_SOL: ",
        constants_1.REGISTERED_POOL_PDA_SOL,
      );
      try {
        yield merkleTreeConfig.registerSolPool(constants_1.POOL_TYPE);
        console.log("Registering sol pool success");
      } catch (e) {
        console.log(e);
      }
    }
  });
}
exports.setUpMerkleTree = setUpMerkleTree;
