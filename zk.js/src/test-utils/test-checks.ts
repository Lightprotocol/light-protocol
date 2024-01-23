import { Connection, PublicKey } from "@solana/web3.js";

export async function checkRentExemption({
  connection,
  account,
}: {
  connection: Connection;
  account: any;
}) {
  const requiredBalance = await connection.getMinimumBalanceForRentExemption(
    account.data.length,
  );
  if (account.lamports < requiredBalance) {
    throw Error(
      `Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`,
    );
  }
}

export async function checkNfInserted(
  pubkeys: { isSigner: boolean; isWritatble: boolean; pubkey: PublicKey }[],
  connection: Connection,
  returnValue: boolean = false,
) {
  for (let i = 0; i < pubkeys.length; i++) {
    const accountInfo = await connection.getAccountInfo(pubkeys[i].pubkey);
    if (!returnValue && accountInfo === null)
      throw new Error("nullifier not inserted");
    else return accountInfo;
  }
}
