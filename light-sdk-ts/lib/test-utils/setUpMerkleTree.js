"use strict";
var __createBinding =
  (this && this.__createBinding) ||
  (Object.create
    ? function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        var desc = Object.getOwnPropertyDescriptor(m, k);
        if (
          !desc ||
          ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)
        ) {
          desc = {
            enumerable: true,
            get: function () {
              return m[k];
            },
          };
        }
        Object.defineProperty(o, k2, desc);
      }
    : function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        o[k2] = m[k];
      });
var __setModuleDefault =
  (this && this.__setModuleDefault) ||
  (Object.create
    ? function (o, v) {
        Object.defineProperty(o, "default", { enumerable: true, value: v });
      }
    : function (o, v) {
        o["default"] = v;
      });
var __importStar =
  (this && this.__importStar) ||
  function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null)
      for (var k in mod)
        if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k))
          __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
  };
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
var __importDefault =
  (this && this.__importDefault) ||
  function (mod) {
    return mod && mod.__esModule ? mod : { default: mod };
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.setUpMerkleTree = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
const chai_1 = require("chai");
const verifier_program_one_1 = __importDefault(
  require("../idls/verifier_program_one")
);
const verifier_program_two_1 = __importDefault(
  require("../idls/verifier_program_two")
);
const verifier_program_zero_1 = __importDefault(
  require("../idls/verifier_program_zero")
);
const index_1 = require("../index");
const merkleTreeConfig_1 = require("../merkleTree/merkleTreeConfig");
function setUpMerkleTree(provider) {
  return __awaiter(this, void 0, void 0, function* () {
    const verifierProgramZero = new anchor.Program(
      verifier_program_zero_1.default,
      index_1.verifierProgramZeroProgramId
    );
    const verifierProgramOne = new anchor.Program(
      verifier_program_one_1.default,
      index_1.verifierProgramOneProgramId
    );
    const verifierProgramTwo = new anchor.Program(
      verifier_program_two_1.default,
      index_1.verifierProgramTwoProgramId
    );
    var merkleTreeAccountInfoInit = yield provider.connection.getAccountInfo(
      index_1.MERKLE_TREE_KEY
    );
    // console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    // console.log("MERKLE_TREE_KEY ", MERKLE_TREE_KEY);
    // console.log("ADMIN_AUTH_KEYPAIR ", ADMIN_AUTH_KEYPAIR);
    if (merkleTreeAccountInfoInit == null) {
      let merkleTreeConfig = new merkleTreeConfig_1.MerkleTreeConfig({
        merkleTreePubkey: index_1.MERKLE_TREE_KEY,
        payer: index_1.ADMIN_AUTH_KEYPAIR,
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
      console.log("AUTHORITY: ", index_1.AUTHORITY);
      console.log("AUTHORITY: ", Array.from(index_1.AUTHORITY.toBytes()));
      console.log(
        "verifierProgramZero.programId: ",
        Array.from(verifierProgramZero.programId.toBytes())
      );
      console.log("MERKLE_TREE_KEY: ", index_1.MERKLE_TREE_KEY.toBase58());
      console.log(
        "MERKLE_TREE_KEY: ",
        Array.from(index_1.MERKLE_TREE_KEY.toBytes())
      );
      // console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
      // console.log("MERKLE_TREE_PDA_TOKEN: ", Array.from(MERKLE_TREE_PDA_TOKEN.toBytes()))
      try {
        const ix = yield merkleTreeConfig.initializeNewMerkleTree();
        (0, chai_1.assert)(
          (yield provider.connection.getTransaction(ix, {
            commitment: "confirmed",
          })) != null,
          "init failed"
        );
      } catch (e) {
        console.log(e);
      }
      console.log("Registering Verifier");
      try {
        yield merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
        console.log("Registering Verifier Zero success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerVerifier(verifierProgramOne.programId);
        console.log("Registering Verifier One success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerVerifier(verifierProgramTwo.programId);
        console.log("Registering Verifier One success");
      } catch (e) {
        console.log(e);
      }
      try {
        yield merkleTreeConfig.registerPoolType(index_1.POOL_TYPE);
        console.log("Registering pool_type success");
      } catch (e) {
        console.log(e);
      }
      console.log("MINT: ", index_1.MINT.toBase58());
      console.log(
        "POOL_TYPE_PDA: ",
        index_1.REGISTERED_POOL_PDA_SPL.toBase58()
      );
      try {
        yield merkleTreeConfig.registerSplPool(index_1.POOL_TYPE, index_1.MINT);
        console.log("Registering spl pool success");
      } catch (e) {
        console.log(e);
      }
      console.log("REGISTERED_POOL_PDA_SOL: ", index_1.REGISTERED_POOL_PDA_SOL);
      try {
        yield merkleTreeConfig.registerSolPool(index_1.POOL_TYPE);
        console.log("Registering sol pool success");
      } catch (e) {
        console.log(e);
      }
    }
  });
}
exports.setUpMerkleTree = setUpMerkleTree;
