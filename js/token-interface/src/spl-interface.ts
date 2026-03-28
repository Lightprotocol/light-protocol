import { Commitment, PublicKey } from "@solana/web3.js";
import { unpackAccount } from "@solana/spl-token";
import { bn, Rpc } from "@lightprotocol/stateless.js";
import BN from "bn.js";
import { deriveSplInterfacePdaWithIndex } from "./constants";

export type SplInterface = {
  mint: PublicKey;
  poolPda: PublicKey;
  tokenProgramId: PublicKey;
  activity?: {
    txs: number;
    amountAdded: BN;
    amountRemoved: BN;
  };
  isInitialized: boolean;
  balance: BN;
  derivationIndex: number;
  bump: number;
};

export async function getSplInterfaces(
  rpc: Rpc,
  mint: PublicKey,
  commitment?: Commitment,
): Promise<SplInterface[]> {
  const addressesAndBumps = Array.from({ length: 5 }, (_, i) =>
    deriveSplInterfacePdaWithIndex(mint, i),
  );

  const accountInfos = await rpc.getMultipleAccountsInfo(
    addressesAndBumps.map(([address]) => address),
    commitment,
  );

  if (accountInfos[0] === null) {
    throw new Error(`SPL interface not found for mint ${mint.toBase58()}.`);
  }

  const parsedInfos = addressesAndBumps.map(([address], i) =>
    accountInfos[i]
      ? unpackAccount(address, accountInfos[i], accountInfos[i].owner)
      : null,
  );

  const tokenProgramId = accountInfos[0].owner;

  return parsedInfos.map((parsedInfo, i) => {
    if (!parsedInfo) {
      return {
        mint,
        poolPda: addressesAndBumps[i][0],
        tokenProgramId,
        activity: undefined,
        balance: bn(0),
        isInitialized: false,
        derivationIndex: i,
        bump: addressesAndBumps[i][1],
      };
    }

    return {
      mint,
      poolPda: parsedInfo.address,
      tokenProgramId,
      activity: undefined,
      balance: bn(parsedInfo.amount.toString()),
      isInitialized: true,
      derivationIndex: i,
      bump: addressesAndBumps[i][1],
    };
  });
}
