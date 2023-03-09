import { relay } from "./relay";
import {
  ADMIN_AUTH_KEYPAIR,
  Relayer,
  Provider,
  confirmConfig,
  createTestAccounts,
  initLookUpTableFromFile,
  MERKLE_TREE_KEY,
  setUpMerkleTree,
  SolMerkleTree,
} from "light-sdk";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";
import express from "express";
const app = express();
const port = 3331;

// Add CORS headers
app.use((req, res, next) => {
  res.header("Access-Control-Allow-Origin", "*");
  res.header(
    "Access-Control-Allow-Headers",
    "Origin, X-Requested-With, Content-Type, Accept"
  );
  next();
});

app.post("/relay", async function (req, res) {
  try {
    if (!req.body.instructions) throw new Error("No instructions provided");
    await relay(req, relayerPayer);
    return res.status(200).json({ status: "ok" });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
});

app.get("/merkletree", async function (req, res) {
  try {
    const provider = await Provider.native(ADMIN_AUTH_KEYPAIR);
    const merkletreeIsInited =
      await provider.provider!.connection.getAccountInfo(MERKLE_TREE_KEY);
    if (!merkletreeIsInited) {
      // await setUpMerkleTree(provider.provider!);
      // console.log("merkletree inited");
      throw new Error("merkletree not inited yet.");
    }

    // console.log("building merkletree...");
    const mt = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: provider.poseidon,
    });
    // console.log("✔️ building merkletree done.");
    provider.solMerkleTree = mt;
    return res.status(200).json({ data: mt });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
});

app.get("/lookuptable", async function (req, res) {
  try {
    const provider = await Provider.native(ADMIN_AUTH_KEYPAIR);
    const LOOK_UP_TABLE = await initLookUpTableFromFile(provider.provider!);
    return res.status(200).json({ data: LOOK_UP_TABLE });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
});

var relayer;
const relayerPayer = ADMIN_AUTH_KEYPAIR;
const relayerFeeRecipient = solana.Keypair.generate();
const relayerFee = new anchor.BN(100000);

const rpcPort = 8899;

(async () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = `http://127.0.0.1:${rpcPort}`; // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    `http://127.0.0.1:${rpcPort}`,
    confirmConfig
  );
  anchor.setProvider(providerAnchor);
  console.log("anchor provider set");

  await createTestAccounts(providerAnchor.connection);
  console.log("test accounts created");
  let LOOK_UP_TABLE = await initLookUpTableFromFile(providerAnchor);
  console.log("lookup table initialized");
  await setUpMerkleTree(providerAnchor);
  /// *** this is not really necessary at this point *** TODO: remove
  console.log("merkletree set up done");
  relayer = new Relayer(
    relayerPayer.publicKey,
    LOOK_UP_TABLE,
    relayerFeeRecipient.publicKey,
    relayerFee
  );

  await providerAnchor!.connection.confirmTransaction(
    await providerAnchor!.connection.requestAirdrop(
      relayer.accounts.relayerRecipient,
      1_000_000
    ),
    "confirmed"
  );
  console.log(
    "Relayer initialized",
    relayer.accounts.relayerPubkey.toBase58(),
    "relayerRecipient: ",
    relayer.accounts.relayerRecipient.toBase58()
  );
})();

app.listen(port, async () => {
  console.log(`Webserver started on port ${port}`);
  console.log("rpc:", process.env.RPC_URL);
});
