import { Command } from "@oclif/core";
import { CustomLoader, getUser } from "../../utils/utils";
import { t } from "tar";

class AccountCommand extends Command {
  static description = "Get the current account details";

  async run() {
    const loader = new CustomLoader(`Fetching account details...`);
    loader.start();
    // TODO: replace with Account.deriveFromKeypair()
    const user = await getUser({
      skipFetchBalance: true,
      localTestRelayer: true,
    });
    this.log(
      `\n\x1b[1mShielded Public Key:\x1b[0m ${user.account.getPublicKey()}`
    );
    this.exit(0);
  }
}

export default AccountCommand;
