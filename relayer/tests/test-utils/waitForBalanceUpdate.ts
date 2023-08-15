import { UserTestAssertHelper, User, sleep } from "@lightprotocol/zk.js";

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
      )
    )
      retries = 0;
    balance = await user.getBalance();
    await sleep(4000);
  }
};
