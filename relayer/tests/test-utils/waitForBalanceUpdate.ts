import { UserTestAssertHelper, User, sleep, BN_0 } from "@lightprotocol/zk.js";

export const waitForBalanceUpdate = async (
  userTestAssertHelper: UserTestAssertHelper,
  user: User,
  retries: number = 15,
) => {
  let balance = await user.getBalance();

  while (retries > 0) {
    retries--;
    if (
      !balance.totalSolBalance.eq(
        userTestAssertHelper.recipient.preShieldedBalance!.totalSolBalance,
      ) &&
      !balance.totalSolBalance.eq(BN_0)
    ) {
      // keeping these for future debugging for now
      // console.log("detected balance change after retries ", retries);
      // console.log("prior balance ", userTestAssertHelper.recipient.preShieldedBalance!.totalSolBalance.toString());
      // console.log("current balance ", balance.totalSolBalance.toString());
      retries = 0;
    }
    balance = await user.getBalance();
    await sleep(4000);
  }
};
