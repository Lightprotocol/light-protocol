import {
  TRANSACTION_MERKLE_TREE_KEY,
  Provider,
  SolMerkleTree,
  updateMerkleTreeForTest,
} from "@lightprotocol/zk.js";
import {
  getLightProvider,
  getRelayer,
  getKeyPairFromEnv,
} from "../utils/provider";

export const initMerkleTree = async (_req: any, res: any) => {
  try {
    const provider: Provider = await getLightProvider();

    const merkletreeIsInited =
      await provider.provider!.connection.getAccountInfo(
        TRANSACTION_MERKLE_TREE_KEY,
      );
    if (!merkletreeIsInited) {
      throw new Error("merkletree not inited yet.");
    }

    const relayer = await getRelayer();

    const indexedTransactions = await relayer.getIndexedTransactions(
      provider.provider!.connection,
    );

    const mt = await SolMerkleTree.build({
      pubkey: TRANSACTION_MERKLE_TREE_KEY,
      poseidon: provider.poseidon,
      indexedTransactions,
      provider: provider.provider,
    });

    provider.solMerkleTree = mt;
    return res.status(200).json({ data: mt });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
};

export const updateMerkleTree = async (_req: any, res: any) => {
  try {
    const provider = await getLightProvider();
    await updateMerkleTreeForTest(getKeyPairFromEnv("KEY_PAIR"), provider.url!);
    return res.status(200).json({ status: "ok" });
  } catch (e) {
    return res.status(500).json({ status: "error", message: e.message });
  }
};
