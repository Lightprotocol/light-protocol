import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { type Rpc } from '@lightprotocol/stateless.js';
import {
    createLightTokenApproveInstruction,
    createLightTokenRevokeInstruction,
} from '../../src/v3/instructions/approve-revoke';

const {
    createApproveInterfaceInstructionsMock,
    createRevokeInterfaceInstructionsMock,
    getMintInterfaceMock,
} = vi.hoisted(() => ({
    createApproveInterfaceInstructionsMock: vi.fn().mockResolvedValue([[]]),
    createRevokeInterfaceInstructionsMock: vi.fn().mockResolvedValue([[]]),
    getMintInterfaceMock: vi.fn().mockResolvedValue({
        mint: { decimals: 9 },
    }),
}));

vi.mock('../../src/v3/instructions/approve-interface', () => ({
    createApproveInterfaceInstructions: createApproveInterfaceInstructionsMock,
    createRevokeInterfaceInstructions: createRevokeInterfaceInstructionsMock,
}));

vi.mock('../../src/v3/get-mint-interface', () => ({
    getMintInterface: getMintInterfaceMock,
}));

import {
    createApproveInterfaceInstructions as unifiedCreateApproveInterfaceInstructions,
    createRevokeInterfaceInstructions as unifiedCreateRevokeInterfaceInstructions,
} from '../../src/v3/unified';

describe('approve/revoke regressions', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('skips mint RPC in unified approve/revoke when decimals are provided', async () => {
        const rpc = {} as Rpc;
        const payer = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const tokenAccount = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        await unifiedCreateApproveInterfaceInstructions(
            rpc,
            payer,
            mint,
            tokenAccount,
            delegate,
            10n,
            owner,
            6,
        );
        await unifiedCreateRevokeInterfaceInstructions(
            rpc,
            payer,
            mint,
            tokenAccount,
            owner,
            6,
        );

        expect(getMintInterfaceMock).not.toHaveBeenCalled();
        expect(createApproveInterfaceInstructionsMock).toHaveBeenCalledWith(
            rpc,
            payer,
            mint,
            tokenAccount,
            delegate,
            10n,
            owner,
            6,
            undefined,
            true,
            undefined,
        );
        expect(createRevokeInterfaceInstructionsMock).toHaveBeenCalledWith(
            rpc,
            payer,
            mint,
            tokenAccount,
            owner,
            6,
            undefined,
            true,
            undefined,
        );
    });

    it('fetches mint decimals in unified approve/revoke when decimals omitted', async () => {
        const rpc = {} as Rpc;
        const payer = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const tokenAccount = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;

        await unifiedCreateApproveInterfaceInstructions(
            rpc,
            payer,
            mint,
            tokenAccount,
            delegate,
            10n,
            owner,
        );
        await unifiedCreateRevokeInterfaceInstructions(
            rpc,
            payer,
            mint,
            tokenAccount,
            owner,
        );

        expect(getMintInterfaceMock).toHaveBeenCalledTimes(2);
        expect(createApproveInterfaceInstructionsMock).toHaveBeenCalledWith(
            rpc,
            payer,
            mint,
            tokenAccount,
            delegate,
            10n,
            owner,
            9,
            undefined,
            true,
            undefined,
        );
        expect(createRevokeInterfaceInstructionsMock).toHaveBeenCalledWith(
            rpc,
            payer,
            mint,
            tokenAccount,
            owner,
            9,
            undefined,
            true,
            undefined,
        );
    });
});
