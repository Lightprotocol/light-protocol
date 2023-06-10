import { Command } from "@oclif/core";
import { getUser } from "../../utils/utils";
import { User } from "@lightprotocol/zk.js";
class AccountCommand extends Command {
  static description = "Get the current account details";

  async run() {
    const user: User = await getUser();
    this.log(`\n\x1b[1mShielded Public Key:\x1b[0m ${await user.account.getPublicKey()}`);
  }
}

export default AccountCommand;
