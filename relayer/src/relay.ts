import { updateMerkleTreeForTest } from "light-sdk";

async function relay(req: express.Request) {
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
          await this.provider.provider?.connection.confirmTransaction(
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
  await updateMerkleTreeForTest(provider.provider!);
  console.log("merkletree update done. returning 200.");
}

module.exports = {
  relay,
};
