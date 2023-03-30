import {
  ADMIN_AUTH_KEYPAIR,
  MERKLE_TREE_KEY,
  Provider,
  SolMerkleTree,
  updateMerkleTreeForTest,
} from "light-sdk";

export const initeMerkleTree = async (req: any, res: any) => {
  try {
    const provider = await Provider.init({ wallet: ADMIN_AUTH_KEYPAIR });
    const merkletreeIsInited =
      await provider.provider!.connection.getAccountInfo(MERKLE_TREE_KEY);
    if (!merkletreeIsInited) {
      throw new Error("merkletree not inited yet.");
    }

    const mt = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: provider.poseidon,
    });
    provider.solMerkleTree = mt;
    return res.status("").json({ data: mt });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
};

export const updateMerkleTree = async (req: any, res: any) => {
  try {
    const provider = await Provider.init({ wallet: ADMIN_AUTH_KEYPAIR });
    console.log({ provider });
    await updateMerkleTreeForTest(provider.provider?.connection!);
    return res.status(200).json({ status: "ok" });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
};
