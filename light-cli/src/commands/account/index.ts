import { Command, Flags } from "@oclif/core";
import { getUser } from "../../utils/utils";
import { User } from "@lightprotocol/zk.js";

class AccountCommand extends Command {
  static description = "Get the current account details";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples: Command.Example[] = ["$ light account"];

  async run() {
    const user: User = await getUser();

    this.log(`\nshielded public key: ${await user.account.getPublicKey()}`);
  }
}

export default AccountCommand;
