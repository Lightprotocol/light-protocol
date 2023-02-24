// basic webserver
const {
  ADMIN_AUTH_KEYPAIR,
  Relayer,
  Provider,
  confirmConfig,
  updateMerkleTreeForTest,
} = require("light-sdk");
const anchor = require("@coral-xyz/anchor");
const solana = require("@solana/web3.js");
const express = require("express");
const { sendTransaction } = require("./sendTransaction");
const { relay } = require("./relay");
const app = express();
// const port = 3000;

// Add CORS headers
app.use((req, res, next) => {
  res.header("Access-Control-Allow-Origin", "*");
  res.header(
    "Access-Control-Allow-Headers",
    "Origin, X-Requested-With, Content-Type, Accept"
  );
  next();
});
// endpoints:
// /relay (unshield, transfer)
app.post("/relay", async function (req, res) {
  try {
    if (!req.body.instructions) throw new Error("No instructions provided");
    await relay(req);
    return res.status(200).json({ status: "ok" });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
});

var relayer: Relayer;
const relayerPayer = ADMIN_AUTH_KEYPAIR;
const relayerFeeRecipient = solana.Keypair.generate();
const relayerFee = new anchor.BN(100000);
(async () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const providerAnchor = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig
  );
  anchor.setProvider(providerAnchor);
  /// *** this is not really necessary at this point *** TODO: remove
  const provider = await Provider.native(relayerPayer);

  relayer = new Relayer(
    relayerPayer.publicKey,
    provider.lookUpTable!,
    relayerFeeRecipient.publicKey,
    relayerFee
  );

  await provider.provider!.connection.confirmTransaction(
    await provider.provider!.connection.requestAirdrop(
      relayer.accounts.relayerRecipient,
      1_000_000
    ),
    "confirmed"
  );
  console.log("Relayer initialized", relayer);
})();
