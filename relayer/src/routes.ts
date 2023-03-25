import { relayerPayer } from "config";
import express from "express";
import { updateMerkleTreeForTest, Provider, MERKLE_TREE_KEY, SolMerkleTree, initLookUpTableFromFile, ADMIN_AUTH_KEYPAIR } from "light-sdk";
import { relay } from "relay";

const router = express.Router();

router.post("/updatemerkletree", async function (req, res) {
    try {
      const provider = await Provider.init(ADMIN_AUTH_KEYPAIR);
      console.log({provider})
      await updateMerkleTreeForTest(provider.provider?.connection!);
      return res.status(200).json({ status: "ok" });
    } catch (e) {
      console.log(e);
      return res.status(500).json({ status: "error" });
    }
  });

  router.post("/updatemerkletree", async function (req, res) {
    try {
      const provider = await Provider.init(ADMIN_AUTH_KEYPAIR);
      console.log({provider})
      await updateMerkleTreeForTest(provider.provider?.connection!);
      return res.status(200).json({ status: "ok" });
    } catch (e) {
      console.log(e);
      return res.status(500).json({ status: "error" });
    }
  });
  
  router.post("/relay", async function (req, res) {
    try {
      if (!req.body.instructions) throw new Error("No instructions provided");
      // TODO: get body.recipientaddress (if spl) - if account doesnt exist create the account (also bumped fee then)
      // inspect data, check that fee is correct
      await relay(req, relayerPayer);
      return res.status(200).json({ status: "ok" });
    } catch (e) {
      console.log(e);
      return res.status(500).json({ status: "error" });
    }
  });

  router.get("/merkletree", async function (req, res) {
    try {
      const provider = await Provider.init(ADMIN_AUTH_KEYPAIR);
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
  
  router.get("/lookuptable", async function (req, res) {
    try {
      const provider = await Provider.init(ADMIN_AUTH_KEYPAIR);
      const LOOK_UP_TABLE = await initLookUpTableFromFile(provider.provider!);
      return res.status(200).json({ data: LOOK_UP_TABLE });
    } catch (e) {
      console.log(e);
      return res.status(500).json({ status: "error" });
    }
  });

  export default router;
