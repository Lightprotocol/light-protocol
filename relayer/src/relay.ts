import { Keypair } from "@solana/web3.js";
import { Provider, updateMerkleTreeForTest } from "light-sdk";
import { sendTransaction } from "./sendTransaction";
<<<<<<< HEAD
export async function relay(req: express.request, relayerPayer: Keypair) {
=======
export async function relay(req: express.Request, relayerPayer: Keypair) {
>>>>>>> e84de8b5 (relayer setting up its own test-validator with runScript.sh)
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
<<<<<<< HEAD
    //@ts-ignore
=======
>>>>>>> e84de8b5 (relayer setting up its own test-validator with runScript.sh)
    await updateMerkleTreeForTest(provider.provider!);
    console.log("merkletree update done. returning 200.");
  } catch (e) {
    console.log("merkletree update failed. ", e);
<<<<<<< HEAD
    throw new Error(`mt update failed: ${e}`);
  }
}
=======
  }
}

// module.exports = {
//   relay,
// };
>>>>>>> e84de8b5 (relayer setting up its own test-validator with runScript.sh)
