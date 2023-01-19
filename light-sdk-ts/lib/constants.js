"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.MERKLE_TREE_HEIGHT = exports.FEE_ASSET = exports.MERKLE_TREE_AUTHORITY_PDA = exports.POOL_TYPE = exports.REGISTERED_POOL_PDA_SOL = exports.REGISTERED_POOL_PDA_SPL_TOKEN = exports.REGISTERED_POOL_PDA_SPL = exports.TOKEN_AUTHORITY = exports.PRE_INSERTED_LEAVES_INDEX = exports.AUTHORITY_ONE = exports.AUTHORITY = exports.REGISTERED_VERIFIER_TWO_PDA = exports.REGISTERED_VERIFIER_ONE_PDA = exports.REGISTERED_VERIFIER_PDA = exports.MERKLE_TREE_KEY = exports.DEFAULT_PROGRAMS = exports.AUTHORITY_SEED = exports.DEFAULT_ZERO = exports.confirmConfig = exports.verifierProgramTwoProgramId = exports.verifierProgramOneProgramId = exports.verifierProgramZeroProgramId = exports.merkleTreeProgramId = exports.TYPE_INIT_DATA = exports.TYPE_SEED = exports.TYPE_PUBKEY = exports.MERKLE_TREE_SIGNER_AUTHORITY = exports.FIELD_SIZE = exports.CONSTANT_SECRET_AUTHKEY = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
const spl_token_1 = require("@solana/spl-token");
const web3_js_1 = require("@solana/web3.js");
exports.CONSTANT_SECRET_AUTHKEY = Uint8Array.from([
    155, 249, 234, 55, 8, 49, 0, 14, 84, 72, 10, 224, 21, 139, 87, 102, 115, 88,
    217, 72, 137, 38, 0, 179, 93, 202, 220, 31, 143, 79, 247, 200,
]);
exports.FIELD_SIZE = new anchor.BN("21888242871839275222246405745257275088548364400416034343698204186575808495617");
exports.MERKLE_TREE_SIGNER_AUTHORITY = new web3_js_1.PublicKey([
    59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42,
    153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225,
]);
exports.TYPE_PUBKEY = { array: ["u8", 32] };
exports.TYPE_SEED = { defined: "&[u8]" };
exports.TYPE_INIT_DATA = { array: ["u8", 642] };
exports.merkleTreeProgramId = new web3_js_1.PublicKey("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
exports.verifierProgramZeroProgramId = new web3_js_1.PublicKey("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");
exports.verifierProgramOneProgramId = new web3_js_1.PublicKey("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");
exports.verifierProgramTwoProgramId = new web3_js_1.PublicKey("GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8");
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
exports.DEFAULT_ZERO = "14522046728041339886521211779101644712859239303505368468566383402165481390632";
exports.AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED");
exports.DEFAULT_PROGRAMS = {
    systemProgram: web3_js_1.SystemProgram.programId,
    tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
    associatedTokenProgram: spl_token_1.ASSOCIATED_TOKEN_PROGRAM_ID,
    rent: web3_js_1.SYSVAR_RENT_PUBKEY,
    clock: web3_js_1.SYSVAR_CLOCK_PUBKEY,
};
// TODO: make account object with important accounts
exports.MERKLE_TREE_KEY = new web3_js_1.PublicKey("DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU");
exports.REGISTERED_VERIFIER_PDA = new web3_js_1.PublicKey("Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn");
exports.REGISTERED_VERIFIER_ONE_PDA = new web3_js_1.PublicKey("CqUS5VyuGscwLMTbfUSAA1grmJYzDAkSR39zpbwW2oV5");
exports.REGISTERED_VERIFIER_TWO_PDA = new web3_js_1.PublicKey("7RCgKAJkaR4Qsgve8D7Q3MrVt8nVY5wdKsmTYVswtJWn");
exports.AUTHORITY = new web3_js_1.PublicKey("KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM");
exports.AUTHORITY_ONE = new web3_js_1.PublicKey("EjGpk73m5KxndbUVXcoT3UQsPLp5eK4h1H8kXVHEbf3f");
exports.PRE_INSERTED_LEAVES_INDEX = new web3_js_1.PublicKey("2MQ7XkirVZZhRQQKcaDiJsrXHCuRHjbu72sUEeW4eZjq");
exports.TOKEN_AUTHORITY = new web3_js_1.PublicKey("GUqBxNbKyB9SBnbBKYR5dajwuWTjTRUhWrZgeFkJND55");
exports.REGISTERED_POOL_PDA_SPL = new web3_js_1.PublicKey("2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4");
exports.REGISTERED_POOL_PDA_SPL_TOKEN = new web3_js_1.PublicKey("2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh");
exports.REGISTERED_POOL_PDA_SOL = new web3_js_1.PublicKey("Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU");
exports.POOL_TYPE = new Uint8Array(32).fill(0);
exports.MERKLE_TREE_AUTHORITY_PDA = new web3_js_1.PublicKey("5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y");
exports.FEE_ASSET = anchor.web3.SystemProgram.programId;
exports.MERKLE_TREE_HEIGHT = 18;
