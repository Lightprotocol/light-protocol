import { Keypair } from "@solana/web3.js";
import { Request } from "express";
import { Provider, updateMerkleTreeForTest } from "light-sdk";
import { sendTransaction } from "./sendTransaction";
export async function relay(
  req: Request, relayerPayer: Keypair
  ) {
  const { instructions } = req.body;
  const provider = await Provider.native(relayerPayer);

  try {
    let ixs = JSON.parse(instructions);
    console.log("PARSED IX:", ixs);
    if (ixs) {
      let tx = "Something went wrong";
      for (let ix in ixs) {
        let txTmp = await sendTransaction(ixs[ix], provider);
        if (txTmp) {
          console.log("tx ::", txTmp);
          await provider.provider?.connection.confirmTransaction(
            txTmp,
            "confirmed"
          );
          tx = txTmp;
        } else {
          throw new Error("send transaction failed");
        }
      }
      return tx;
    } else {
      throw new Error("No parameters provided");
    }
  } catch (e) {
    console.log(e);
  }
  //TODO: add a check mechanism here await tx.checkBalances();
  console.log("confirmed tx, updating merkletree...");
  try {
    await updateMerkleTreeForTest(provider.provider?.connection!);
    console.log("merkletree update done. returning 200.");
  } catch (e) {
    console.log("merkletree update failed. ", e);
  }
}
