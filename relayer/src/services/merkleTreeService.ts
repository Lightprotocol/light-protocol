import {
  ADMIN_AUTH_KEYPAIR,
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

export const buildMerkleTree = async (req: any, res: any) => {
  try {
    const provider: Provider = await getLightProvider();

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

export const updateMerkleTree = async (req: any, res: any) => {
  console.log("Relayer updating merkle tree");
  try {
    const provider = await getLightProvider();
    await updateMerkleTreeForTest(getKeyPairFromEnv("KEY_PAIR"), provider.url!);
    return res.status(200).json({ status: "ok" });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
};
