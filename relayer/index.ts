// basic webserver
import { ADMIN_AUTH_KEYPAIR, getLightInstance, Relayer } from "light-sdk";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";
import * as express from "express";
const app = express();
const port = 3000;

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
  await relay(req, res);
  return res.status(200).json({ status: "ok" });
});

//
var relayer: Relayer;
(async () => {
  const relayerPayer = ADMIN_AUTH_KEYPAIR;
  const relayerFeeRecipient = solana.Keypair.generate();
  const relayerFee = new anchor.BN(100000);
  const lightInstance = await getLightInstance();

  relayer = new Relayer(
    relayerPayer.publicKey,
    lightInstance.lookUpTable!,
    relayerFeeRecipient.publicKey,
    relayerFee
  );
  console.log("Relayer initialized", relayer);
})();

async function relay(req: express.Request, res: express.Response) {
  const { tx, sig } = req.body;
  const lightInstance = await getLightInstance();

  // const txSig = await relayer.relay(tx, sig);

  // "parse tx, sign it, and send it to the network"

  res.send(txSig);
}
