import { Command, ux } from "@oclif/core";
import { BN, Program } from "@coral-xyz/anchor";
import { CustomLoader, setAnchorProvider } from "../../utils/utils";
import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  merkleTreeProgramId,
} from "@lightprotocol/zk.js";

class AssetPoolListCommand extends Command {
  static description = "List asset pools.";

  static examples = ["light asset-pool:list"];

  async run() {
    const loader = new CustomLoader("Listing pool accounts");
    loader.start();

    const provider = await setAnchorProvider();
    const merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider,
    );

    const assetPoolsAccounts =
      await merkleTreeProgram.account.registeredAssetPool.all();

    loader.stop(false);
    ux.table(assetPoolsAccounts, {
      index: {
        header: "Index",
        get: (account) => account.account.index.toString(),
      },
      type: {
        header: "Type",
        get: (account) => new BN(account.account.poolType).toString(),
      },
      publicKey: {
        header: "Public key",
        get: (account) => account.account.assetPoolPubkey.toBase58(),
      },
    });
  }
}

export default AssetPoolListCommand;
