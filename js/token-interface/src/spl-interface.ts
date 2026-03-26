import { Commitment, PublicKey } from '@solana/web3.js';
import { unpackAccount } from '@solana/spl-token';
import { bn, Rpc } from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { deriveSplPoolPdaWithIndex } from './constants';

export type SplPoolInfo = {
    mint: PublicKey;
    splPoolPda: PublicKey;
    tokenProgram: PublicKey;
    activity?: {
        txs: number;
        amountAdded: BN;
        amountRemoved: BN;
    };
    isInitialized: boolean;
    balance: BN;
    poolIndex: number;
    bump: number;
};

export async function getSplPoolInfos(
    rpc: Rpc,
    mint: PublicKey,
    commitment?: Commitment,
): Promise<SplPoolInfo[]> {
    const addressesAndBumps = Array.from({ length: 5 }, (_, i) =>
        deriveSplPoolPdaWithIndex(mint, i),
    );

    const accountInfos = await rpc.getMultipleAccountsInfo(
        addressesAndBumps.map(([address]) => address),
        commitment,
    );

    if (accountInfos[0] === null) {
        throw new Error(`SPL pool not found for mint ${mint.toBase58()}.`);
    }

    const parsedInfos = addressesAndBumps.map(([address], i) =>
        accountInfos[i] ? unpackAccount(address, accountInfos[i], accountInfos[i].owner) : null,
    );

    const tokenProgram = accountInfos[0].owner;

    return parsedInfos.map((parsedInfo, i) => {
        if (!parsedInfo) {
            return {
                mint,
                splPoolPda: addressesAndBumps[i][0],
                tokenProgram,
                activity: undefined,
                balance: bn(0),
                isInitialized: false,
                poolIndex: i,
                bump: addressesAndBumps[i][1],
            };
        }

        return {
            mint,
            splPoolPda: parsedInfo.address,
            tokenProgram,
            activity: undefined,
            balance: bn(parsedInfo.amount.toString()),
            isInitialized: true,
            poolIndex: i,
            bump: addressesAndBumps[i][1],
        };
    });
}
