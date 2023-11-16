import { Command } from "@oclif/core";
import { CustomLoader, setAnchorProvider } from "../../utils";
import { initLookUpTable, useWallet } from "@lightprotocol/zk.js";
import { PathOrFileDescriptor, readFileSync } from "fs";
import { Keypair } from "@solana/web3.js";

class InitializeCommand extends Command {
  static description = "Initialize new lookup table.";

  static examples = ["light lookup-table:initialize"];

  async run() {
    const loader = new CustomLoader("Initializing new lookup table");
    loader.start();

    const anchorProvider = await setAnchorProvider();

    const privkey = JSON.parse(
      readFileSync(process.env.ANCHOR_WALLET as PathOrFileDescriptor, "utf8")
    );
    const payer = Keypair.fromSecretKey(Uint8Array.from(privkey));

    await initLookUpTable(
      useWallet(payer, process.env.ANCHOR_PROVIDER_URL, true, "confirmed"),
      anchorProvider
    );
    this.log("Lookup table initialized successfully \x1b[32m✔\x1b[0m");
    loader.stop(false);
  }
}

export default InitializeCommand;
