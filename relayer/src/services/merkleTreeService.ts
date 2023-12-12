import {
  MerkleTreeConfig,
  Provider,
  SolMerkleTree,
} from "@lightprotocol/zk.js";
import { getLightProvider, getRelayer } from "../utils/provider";

export const buildMerkleTree = async (_req: any, res: any) => {
  try {
    const provider: Provider = await getLightProvider();
    const transactionMerkleTreePda =
      MerkleTreeConfig.getTransactionMerkleTreePda();

    const relayer = getRelayer();

    const indexedTransactions = await relayer.getIndexedTransactions(
      provider.provider!.connection,
    );

    const mt = await SolMerkleTree.build({
      pubkey: transactionMerkleTreePda,
      hasher: provider.hasher,
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
