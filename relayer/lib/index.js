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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const relay_1 = require("./relay");
const light_sdk_1 = require("light-sdk");
const anchor = __importStar(require("@coral-xyz/anchor"));
const solana = __importStar(require("@solana/web3.js"));
const express_1 = __importDefault(require("express"));
const app = (0, express_1.default)();
const port = 3331;
// Add CORS headers
app.use((req, res, next) => {
    res.header("Access-Control-Allow-Origin", "*");
    res.header("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept");
    next();
});
// endpoints:
app.post("/relay", function (req, res) {
    return __awaiter(this, void 0, void 0, function* () {
        // throw new Error("/relayer endpoint not implemented yet.");
        try {
            if (!req.body.instructions)
                throw new Error("No instructions provided");
            yield (0, relay_1.relay)(req, relayerPayer);
            return res.status(200).json({ status: "ok" });
        }
        catch (e) {
            console.log(e);
            return res.status(500).json({ status: "error" });
        }
    });
});
app.get("/merkletree", function (req, res) {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            const provider = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            const merkletreeIsInited = yield provider.provider.connection.getAccountInfo(light_sdk_1.MERKLE_TREE_KEY);
            if (!merkletreeIsInited) {
                // await setUpMerkleTree(provider.provider!);
                // console.log("merkletree inited");
                throw new Error("merkletree not inited yet.");
            }
            console.log("building merkletree...");
            const mt = yield light_sdk_1.SolMerkleTree.build({
                pubkey: light_sdk_1.MERKLE_TREE_KEY,
                poseidon: provider.poseidon,
            });
            console.log("✔️ building merkletree done.");
            provider.solMerkleTree = mt;
            return res.status(200).json({ data: mt });
        }
        catch (e) {
            console.log(e);
            return res.status(500).json({ status: "error" });
        }
    });
});
app.get("/lookuptable", function (req, res) {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            const provider = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            const LOOK_UP_TABLE = yield (0, light_sdk_1.initLookUpTableFromFile)(provider.provider);
            return res.status(200).json({ data: LOOK_UP_TABLE });
        }
        catch (e) {
            console.log(e);
            return res.status(500).json({ status: "error" });
        }
    });
});
var relayer;
const relayerPayer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
const relayerFeeRecipient = solana.Keypair.generate();
const relayerFee = new anchor.BN(100000);
const rpcPort = 8899;
(() => __awaiter(void 0, void 0, void 0, function* () {
    process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
    process.env.ANCHOR_PROVIDER_URL = `http://127.0.0.1:${rpcPort}`; // runscript starts dedicated validator on this port.
    const providerAnchor = anchor.AnchorProvider.local(`http://127.0.0.1:${rpcPort}`, light_sdk_1.confirmConfig);
    anchor.setProvider(providerAnchor);
    console.log("anchor provider set");
    yield (0, light_sdk_1.createTestAccounts)(providerAnchor.connection);
    console.log("test accounts created");
    let LOOK_UP_TABLE = yield (0, light_sdk_1.initLookUpTableFromFile)(providerAnchor);
    console.log("lookup table initialized");
    yield (0, light_sdk_1.setUpMerkleTree)(providerAnchor);
    /// *** this is not really necessary at this point *** TODO: remove
    console.log("merkletree set up done");
    relayer = new light_sdk_1.Relayer(relayerPayer.publicKey, LOOK_UP_TABLE, relayerFeeRecipient.publicKey, relayerFee);
    yield providerAnchor.connection.confirmTransaction(yield providerAnchor.connection.requestAirdrop(relayer.accounts.relayerRecipient, 1000000), "confirmed");
    console.log("Relayer initialized", relayer.accounts.relayerPubkey.toBase58(), "relayerRecipient: ", relayer.accounts.relayerRecipient.toBase58());
}))();
app.listen(port, () => __awaiter(void 0, void 0, void 0, function* () {
    console.log(`Webserver started on port ${port}`);
    console.log("rpc:", process.env.RPC_URL);
}));
