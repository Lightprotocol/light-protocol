"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MAX_MESSAGE_SIZE = exports.RELAYER_FEE = exports.SIGN_MESSAGE = exports.UTXO_FEE_ASSET_MINIMUM = exports.UTXO_MERGE_MAXIMUM = exports.UTXO_MERGE_THRESHOLD = exports.MERKLE_TREE_HEIGHT = exports.FEE_ASSET = exports.TESTNET_LOOK_UP_TABLE = exports.MERKLE_TREE_AUTHORITY_PDA = exports.POOL_TYPE = exports.REGISTERED_POOL_PDA_SOL = exports.REGISTERED_POOL_PDA_SPL_TOKEN = exports.REGISTERED_POOL_PDA_SPL = exports.TOKEN_AUTHORITY = exports.PRE_INSERTED_LEAVES_INDEX = exports.AUTHORITY_ONE = exports.AUTHORITY = exports.REGISTERED_VERIFIER_TWO_PDA = exports.REGISTERED_VERIFIER_ONE_PDA = exports.REGISTERED_VERIFIER_PDA = exports.TOKEN_ACCOUNT_FEE = exports.MINIMUM_LAMPORTS = exports.DEFAULT_PROGRAMS = exports.AUTHORITY_SEED = exports.DEFAULT_ZERO = exports.DEFAULT_PRIVATE_KEY = exports.ENCRYPTED_UNCOMPRESSED_UTXO_BYTES_LENGTH = exports.UNCOMPRESSED_UTXO_BYTES_LENGTH = exports.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH = exports.ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH = exports.COMPRESSED_UTXO_BYTES_LENGTH = exports.confirmConfig = exports.VERIFIER_PUBLIC_KEYS = exports.MAX_U64 = exports.LOOK_UP_TABLE = exports.verifierProgramTwoProgramId = exports.verifierProgramOneProgramId = exports.verifierProgramZeroProgramId = exports.verifierProgramStorageProgramId = exports.merkleTreeProgramId = exports.TYPE_INIT_DATA = exports.TYPE_SEED = exports.TYPE_PUBKEY = exports.MERKLE_TREE_SIGNER_AUTHORITY = exports.FIELD_SIZE = exports.CONSTANT_SECRET_AUTHKEY = exports.BN_2 = exports.BN_1 = exports.BN_0 = void 0;
exports.TRANSACTION_MERKLE_TREE_SWITCH_TRESHOLD = exports.TOKEN_PUBKEY_SYMBOL = exports.TOKEN_REGISTRY = exports.SOL_DECIMALS = void 0;
const tslib_1 = require("tslib");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const anchor_1 = require("@coral-xyz/anchor");
const spl_token_1 = require("@solana/spl-token");
const web3_js_1 = require("@solana/web3.js");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
exports.BN_0 = new anchor.BN(0);
exports.BN_1 = new anchor.BN(1);
exports.BN_2 = new anchor.BN(2);
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
exports.verifierProgramStorageProgramId = new web3_js_1.PublicKey("DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj");
exports.verifierProgramZeroProgramId = new web3_js_1.PublicKey("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");
exports.verifierProgramOneProgramId = new web3_js_1.PublicKey("J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc");
exports.verifierProgramTwoProgramId = new web3_js_1.PublicKey("2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86");
exports.LOOK_UP_TABLE = new web3_js_1.PublicKey("DyZnme4h32E66deCvsAV6pVceVw8s6ucRhNcwoofVCem");
exports.MAX_U64 = new anchor.BN("18446744073709551615");
exports.VERIFIER_PUBLIC_KEYS = [
    exports.verifierProgramZeroProgramId,
    exports.verifierProgramOneProgramId,
    exports.verifierProgramTwoProgramId,
    exports.verifierProgramStorageProgramId,
];
exports.confirmConfig = {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
};
exports.COMPRESSED_UTXO_BYTES_LENGTH = 96 + anchor_1.ACCOUNT_DISCRIMINATOR_SIZE;
exports.ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH = parseInt(((exports.COMPRESSED_UTXO_BYTES_LENGTH + 16) / 16).toString()) * 16;
exports.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH = exports.COMPRESSED_UTXO_BYTES_LENGTH + 16;
exports.UNCOMPRESSED_UTXO_BYTES_LENGTH = exports.COMPRESSED_UTXO_BYTES_LENGTH + 2 * 32;
exports.ENCRYPTED_UNCOMPRESSED_UTXO_BYTES_LENGTH = exports.UNCOMPRESSED_UTXO_BYTES_LENGTH + 16;
exports.DEFAULT_PRIVATE_KEY = bytes_1.bs58.encode(new Uint8Array(32).fill(0));
exports.DEFAULT_ZERO = "14522046728041339886521211779101644712859239303505368468566383402165481390632";
exports.AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED");
exports.DEFAULT_PROGRAMS = {
    systemProgram: web3_js_1.SystemProgram.programId,
    tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
    associatedTokenProgram: spl_token_1.ASSOCIATED_TOKEN_PROGRAM_ID,
    rent: web3_js_1.SYSVAR_RENT_PUBKEY,
    clock: web3_js_1.SYSVAR_CLOCK_PUBKEY,
};
// recommented minimum amount of lamports to be able to pay for transaction fees
// needs to be more than 890_880 to be rentexempt
exports.MINIMUM_LAMPORTS = new anchor.BN(890880 + 150000);
exports.TOKEN_ACCOUNT_FEE = new anchor.BN(1461600 + 5000);
exports.REGISTERED_VERIFIER_PDA = new web3_js_1.PublicKey("Eo3jtUstuMCvapqXdWiYvoUJS1PJDtKVf6LdsMPdyoNn");
exports.REGISTERED_VERIFIER_ONE_PDA = new web3_js_1.PublicKey("9Q5JQPJEqC71R3jTnrnrSEhjMouCVf2dNjURp1L25Wnr");
exports.REGISTERED_VERIFIER_TWO_PDA = new web3_js_1.PublicKey("DRwtrkmoUe9VD4T2KRN2A41jqtHgdDeEH8b3sXu7dHVW");
exports.AUTHORITY = new web3_js_1.PublicKey("KitaXMAzb8GPZcc6NW6mE7P6gV2fY3Bp8NqZWfeUwqM");
exports.AUTHORITY_ONE = new web3_js_1.PublicKey("6n2eREPP6bMLLYVJSGcSCULFy7u2WDrx3v5GJR7bByMa");
exports.PRE_INSERTED_LEAVES_INDEX = new web3_js_1.PublicKey("2MQ7XkirVZZhRQQKcaDiJsrXHCuRHjbu72sUEeW4eZjq");
exports.TOKEN_AUTHORITY = new web3_js_1.PublicKey("GUqBxNbKyB9SBnbBKYR5dajwuWTjTRUhWrZgeFkJND55");
exports.REGISTERED_POOL_PDA_SPL = new web3_js_1.PublicKey("2q4tXrgpsDffibmjfTGHU1gWCjYUfhwFnMyLX6dAhhr4");
exports.REGISTERED_POOL_PDA_SPL_TOKEN = new web3_js_1.PublicKey("2mobV36eNyFGaMTKCHW1Jeoq64tUGuXqA4zGtY8SbxKh");
exports.REGISTERED_POOL_PDA_SOL = new web3_js_1.PublicKey("Eti4Rjkx7ow88XkaFbxRStmwadTp8p9J2nSv7NhtuqDU");
exports.POOL_TYPE = new Array(32).fill(0);
exports.MERKLE_TREE_AUTHORITY_PDA = new web3_js_1.PublicKey("5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y");
exports.TESTNET_LOOK_UP_TABLE = new web3_js_1.PublicKey("64Act4KKVEHFAnjaift46c4ZkutkmT4msN1esSnE6gaJ");
exports.FEE_ASSET = anchor.web3.SystemProgram.programId;
exports.MERKLE_TREE_HEIGHT = 18;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
exports.UTXO_MERGE_THRESHOLD = 20; // 7
exports.UTXO_MERGE_MAXIMUM = 10;
exports.UTXO_FEE_ASSET_MINIMUM = 100000;
exports.SIGN_MESSAGE = "IMPORTANT:\nThe application will be able to spend \nyour shielded assets. \n\nOnly sign the message if you trust this\n application.\n\n View all verified integrations here: \n'https://docs.lightprotocol.com/partners'";
exports.RELAYER_FEE = new anchor.BN(100000);
// TODO: change once we have adapted getInstructions for repeating instructions
exports.MAX_MESSAGE_SIZE = 800;
exports.SOL_DECIMALS = new anchor.BN(1e9);
exports.TOKEN_REGISTRY = new Map([
    [
        "SOL",
        {
            symbol: "SOL",
            decimals: exports.SOL_DECIMALS,
            isNft: false,
            isNative: true,
            mint: web3_js_1.SystemProgram.programId,
        },
    ],
    [
        "USDC",
        {
            symbol: "USDC",
            decimals: new anchor.BN(1e2),
            isNft: false,
            isNative: false,
            // copied from MINT (test-utils)
            mint: new web3_js_1.PublicKey("ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe"),
        },
    ],
]);
exports.TOKEN_PUBKEY_SYMBOL = new Map([
    ["11111111111111111111111111111111", "SOL"],
    [
        "ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe",
        "USDC",
    ],
]);
/**
 * Treshold after which the currently used transaction Merkle tree is switched
 * to the next one. The limit of each merkle tree is 256k, but we want to have
 * a margin.
 */
exports.TRANSACTION_MERKLE_TREE_SWITCH_TRESHOLD = new anchor.BN(255000);
//# sourceMappingURL=constants.js.map