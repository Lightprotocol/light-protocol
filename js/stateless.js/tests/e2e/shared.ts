import {
    ADDRESS_QUEUE_ROLLOVER_FEE,
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    bn,
    StateTreeInfo,
    STATE_MERKLE_TREE_NETWORK_FEE,
    ADDRESS_TREE_NETWORK_FEE,
} from '../../src';

import { TreeType } from '../../src';

import { Rpc } from '../../src';

// Helper function to create different types of state trees
export async function getStateTreeInfoByTypeForTest(
    rpc: Rpc,
    type: TreeType,
): Promise<StateTreeInfo> {
    const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
    switch (type) {
        case TreeType.StateV1:
            return stateTreeInfo[0];
        case TreeType.StateV2:
            return stateTreeInfo[2];

        default:
            throw new Error(`Unknown tree type: ${type}`);
    }
}

export function txFees(
    txs: {
        in: number;
        out: number;
        addr?: number;
        base?: number;
    }[],
): number {
    let totalFee = bn(0);

    txs.forEach(tx => {
        const solanaBaseFee = tx.base === 0 ? bn(0) : bn(tx.base || 5000);

        /// Fee per output
        const stateOutFee = STATE_MERKLE_TREE_ROLLOVER_FEE.mul(bn(tx.out));

        /// Fee per new address created
        const addrFee = tx.addr
            ? ADDRESS_QUEUE_ROLLOVER_FEE.mul(bn(tx.addr))
            : bn(0);

        /// Fee if the tx nullifies at least one input account
        const networkInFee =
            tx.in || tx.out ? STATE_MERKLE_TREE_NETWORK_FEE : bn(0);

        /// Fee if the tx creates at least one address
        const networkAddressFee = tx.addr ? ADDRESS_TREE_NETWORK_FEE : bn(0);

        totalFee = totalFee.add(
            solanaBaseFee
                .add(stateOutFee)
                .add(addrFee)
                .add(networkInFee)
                .add(networkAddressFee),
        );
    });

    return totalFee.toNumber();
}

const DEFAULT_BASE_FEE = 5000;
const STATE_OUT_FEE_MULTIPLIER = 1;
const NETWORK_FEE = 5000;
const ADDITIONAL_NETWORK_ADDRESS_FEE = 5000;

// TODO: add unit tests.
export function txFeesV2Accounts(
    txs: {
        in: number;
        out: number;
        addr?: number;
        base?: number;
    }[],
): number {
    let totalFee = bn(0);

    txs.forEach(tx => {
        const solanaBaseFee =
            tx.base === 0 ? bn(0) : bn(tx.base || DEFAULT_BASE_FEE);

        /// Fee per output
        const stateOutFee = bn(STATE_OUT_FEE_MULTIPLIER).mul(bn(tx.out));

        /// Fee per new address created
        const addrFee = tx.addr
            ? ADDRESS_QUEUE_ROLLOVER_FEE.mul(bn(tx.addr))
            : bn(0);

        /// Network fee if any account is created or modified
        const networkFee = tx.in || tx.out || tx.addr ? bn(NETWORK_FEE) : bn(0);

        /// Additional network fee if an address is created
        const additionalNetworkAddressFee = tx.addr
            ? bn(ADDITIONAL_NETWORK_ADDRESS_FEE)
            : bn(0);

        totalFee = totalFee.add(
            solanaBaseFee
                .add(stateOutFee)
                .add(addrFee)
                .add(networkFee)
                .add(additionalNetworkAddressFee),
        );
    });

    return totalFee.toNumber();
}
