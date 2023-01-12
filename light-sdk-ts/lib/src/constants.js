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
var __importDefault =
  (this && this.__importDefault) ||
  function (mod) {
    return mod && mod.__esModule ? mod : { default: mod };
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.FEE_ASSET =
  exports.MINT_CIRCUIT =
  exports.KEYPAIR_PRIVKEY =
  exports.MERKLE_TREE_AUTHORITY_PDA =
  exports.POOL_TYPE =
  exports.REGISTERED_POOL_PDA_SOL =
  exports.REGISTERED_POOL_PDA_SPL_TOKEN =
  exports.REGISTERED_POOL_PDA_SPL =
  exports.TOKEN_AUTHORITY =
  exports.PRE_INSERTED_LEAVES_INDEX =
  exports.AUTHORITY_ONE =
  exports.AUTHORITY =
  exports.REGISTERED_VERIFIER_TWO_PDA =
  exports.REGISTERED_VERIFIER_ONE_PDA =
  exports.REGISTERED_VERIFIER_PDA =
  exports.MERKLE_TREE_KEY =
  exports.RECIPIENT_TOKEN_ACCOUNT =
  exports.USER_TOKEN_ACCOUNT =
  exports.ENCRYPTION_KEYPAIR =
  exports.recipientTokenAccount =
  exports.userTokenAccount =
  exports.DEFAULT_PROGRAMS =
  exports.AUTHORITY_SEED =
  exports.ADMIN_AUTH_KEYPAIR =
  exports.ADMIN_AUTH_KEY =
  exports.MINT =
  exports.MINT_PRIVATE_KEY =
  exports.MERKLE_TREE_INIT_AUTHORITY =
  exports.PRIVATE_KEY =
  exports.DEFAULT_ZERO =
  exports.confirmConfig =
  exports.verifierProgramTwo =
  exports.verifierProgramOne =
  exports.verifierProgramZero =
  exports.merkleTreeProgram =
  exports.verifierProgramTwoProgramId =
  exports.verifierProgramOneProgramId =
  exports.verifierProgramZeroProgramId =
  exports.merkleTreeProgramId =
  exports.TYPE_INIT_DATA =
  exports.TYPE_SEED =
  exports.TYPE_PUBKEY =
  exports.MERKLE_TREE_SIGNER_AUTHORITY =
  exports.FIELD_SIZE_ETHERS =
  exports.FIELD_SIZE =
  exports.ASSET_1 =
  exports.ASSET_1_ORG =
    void 0;
// TODO: clean this up
// TODO: move test only stuff to other file
const solana = require("@solana/web3.js");
const anchor = __importStar(require("@project-serum/anchor"));
const anchor_1 = require("@project-serum/anchor");
const spl_token_1 = require("@solana/spl-token");
const verifier_program_two_1 = __importDefault(
  require("../idls/verifier_program_two")
);
const verifier_program_one_1 = __importDefault(
  require("../idls/verifier_program_one")
);
const verifier_program_zero_1 = __importDefault(
  require("../idls/verifier_program_zero")
);
const merkle_tree_program_1 = __importDefault(
  require("../idls/merkle_tree_program")
);
const web3_js_1 = require("@solana/web3.js");
const ethers_1 = require("ethers");
const bn_js_1 = require("bn.js");
exports.ASSET_1_ORG = new anchor.web3.Account();
exports.ASSET_1 = new anchor.BN(
  exports.ASSET_1_ORG.publicKey._bn.toBuffer(32).slice(0, 31)
);
exports.FIELD_SIZE = new anchor.BN(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617"
);
exports.FIELD_SIZE_ETHERS = ethers_1.BigNumber.from(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617"
);
exports.MERKLE_TREE_SIGNER_AUTHORITY = new solana.PublicKey([
  59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42,
  153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225,
]);
exports.TYPE_PUBKEY = { array: ["u8", 32] };
exports.TYPE_SEED = { defined: "&[u8]" };
exports.TYPE_INIT_DATA = { array: ["u8", 642] };
exports.merkleTreeProgramId = new web3_js_1.PublicKey(
  "JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6"
);
exports.verifierProgramZeroProgramId = new web3_js_1.PublicKey(
  "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i"
);
exports.verifierProgramOneProgramId = new web3_js_1.PublicKey(
  "3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL"
);
exports.verifierProgramTwoProgramId = new web3_js_1.PublicKey(
  "GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8"
);
exports.merkleTreeProgram = new anchor_1.Program(
  merkle_tree_program_1.default,
  exports.merkleTreeProgramId
);
exports.verifierProgramZero = new anchor_1.Program(
  verifier_program_zero_1.default,
  exports.verifierProgramZeroProgramId
);
exports.verifierProgramOne = new anchor_1.Program(
  verifier_program_one_1.default,
  exports.verifierProgramOneProgramId
);
exports.verifierProgramTwo = new anchor_1.Program(
  verifier_program_two_1.default,
  exports.verifierProgramTwoProgramId
);
exports.confirmConfig = {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
};
// TODO: reactivate this
// const constants:any = {};
// IDL.constants.map((item) => {
//   if(_.isEqual(item.type, TYPE_SEED)) {
//     constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
//   } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
//   {
//     constants[item.name] = JSON.parse(item.value)
//   }
// });
exports.DEFAULT_ZERO =
  "14522046728041339886521211779101644712859239303505368468566383402165481390632";
exports.PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187,
  228, 110, 146, 97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226,
  251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121,
  176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
exports.MERKLE_TREE_INIT_AUTHORITY = [
  2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105,
  144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
exports.MINT_PRIVATE_KEY = new Uint8Array([
  194, 220, 38, 233, 140, 177, 44, 255, 131, 7, 129, 209, 20, 230, 130, 41, 128,
  186, 233, 161, 10, 77, 134, 70, 34, 141, 30, 246, 145, 69, 69, 35, 14, 129,
  15, 86, 229, 176, 155, 3, 8, 217, 125, 97, 221, 115, 252, 160, 127, 236, 37,
  229, 116, 84, 111, 6, 5, 182, 141, 86, 7, 23, 246, 215,
]);
exports.MINT = new solana.PublicKey([
  14, 129, 15, 86, 229, 176, 155, 3, 8, 217, 125, 97, 221, 115, 252, 160, 127,
  236, 37, 229, 116, 84, 111, 6, 5, 182, 141, 86, 7, 23, 246, 215,
]);
exports.ADMIN_AUTH_KEY = new solana.PublicKey(
  new Uint8Array(exports.MERKLE_TREE_INIT_AUTHORITY)
);
exports.ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(
  new Uint8Array(exports.PRIVATE_KEY)
);
exports.AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED");
exports.DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
  associatedTokenProgram: spl_token_1.ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};
exports.userTokenAccount = new solana.PublicKey(
  "CfyD2mSomGrjnyMKWrgNEk1ApaaUvKRDsnQngGkCVTFk"
);
exports.recipientTokenAccount = new solana.PublicKey(
  "6RtYrpXTyH98dvTf9ufivkyDG8mF48oMDbhiRW9r5KjD"
);
exports.ENCRYPTION_KEYPAIR = {
  PublicKey: new Uint8Array([
    45, 218, 154, 197, 141, 144, 160, 47, 100, 67, 150, 144, 22, 128, 18, 23,
    104, 2, 176, 172, 176, 238, 235, 14, 118, 139, 22, 151, 86, 26, 136, 84,
  ]),
  secretKey: new Uint8Array([
    246, 19, 199, 8, 120, 165, 210, 59, 113, 102, 63, 98, 185, 252, 50, 12, 35,
    89, 71, 60, 189, 251, 109, 89, 92, 74, 233, 128, 148, 50, 243, 162,
  ]),
};
exports.USER_TOKEN_ACCOUNT = solana.Keypair.fromSecretKey(
  new Uint8Array([
    213, 170, 148, 167, 77, 163, 59, 129, 233, 8, 59, 40, 203, 223, 53, 122,
    242, 95, 5, 9, 102, 7, 50, 204, 117, 74, 106, 114, 106, 225, 37, 203, 222,
    28, 100, 182, 147, 102, 98, 110, 249, 219, 249, 24, 50, 149, 18, 75, 184,
    183, 246, 83, 13, 66, 226, 103, 241, 88, 135, 253, 226, 32, 41, 186,
  ])
);
exports.RECIPIENT_TOKEN_ACCOUNT = solana.Keypair.fromSecretKey(
  new Uint8Array([
    242, 215, 38, 124, 190, 226, 219, 18, 34, 111, 222, 22, 105, 139, 168, 50,
    113, 227, 43, 76, 83, 234, 5, 93, 242, 182, 158, 40, 141, 213, 16, 229, 254,
    86, 86, 250, 191, 38, 191, 237, 255, 198, 0, 140, 74, 85, 247, 85, 30, 34,
    76, 42, 114, 252, 102, 230, 216, 107, 44, 225, 133, 40, 17, 6,
  ])
);
exports.MERKLE_TREE_KEY = new solana.PublicKey(
  "DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU"
);
exports.REGISTERED_VERIFIER_PDA = new solana.PublicKey(
  "Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn"
);
exports.REGISTERED_VERIFIER_ONE_PDA = new solana.PublicKey(
  "CqUS5VyuGscwLMTbfUSAA1grmJYzDAkSR39zpbwW2oV5"
);
exports.REGISTERED_VERIFIER_TWO_PDA = new solana.PublicKey(
  "7RCgKAJkaR4Qsgve8D7Q3MrVt8nVY5wdKsmTYVswtJWn"
);
exports.AUTHORITY = new solana.PublicKey(
  "KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM"
);
exports.AUTHORITY_ONE = new solana.PublicKey(
  "EjGpk73m5KxndbUVXcoT3UQsPLp5eK4h1H8kXVHEbf3f"
);
exports.PRE_INSERTED_LEAVES_INDEX = new solana.PublicKey(
  "2MQ7XkirVZZhRQQKcaDiJsrXHCuRHjbu72sUEeW4eZjq"
);
exports.TOKEN_AUTHORITY = new solana.PublicKey(
  "GUqBxNbKyB9SBnbBKYR5dajwuWTjTRUhWrZgeFkJND55"
);
exports.REGISTERED_POOL_PDA_SPL = new solana.PublicKey(
  "2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4"
);
exports.REGISTERED_POOL_PDA_SPL_TOKEN = new solana.PublicKey(
  "2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh"
);
exports.REGISTERED_POOL_PDA_SOL = new solana.PublicKey(
  "Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU"
);
exports.POOL_TYPE = new Uint8Array(32).fill(0);
exports.MERKLE_TREE_AUTHORITY_PDA = new solana.PublicKey(
  "5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y"
);
exports.KEYPAIR_PRIVKEY = new bn_js_1.BN(
  "d67b402d88fe6eb59004f4ab53b06a4b9dc72c74a05e60c31a07148eafa95896",
  "hex"
);
exports.MINT_CIRCUIT = new anchor.BN(
  exports.MINT._bn.toBuffer(32).slice(0, 31)
);
exports.FEE_ASSET = new anchor.BN(
  anchor.web3.SystemProgram.programId._bn.toBuffer(32).slice(0, 31)
); //new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE)
