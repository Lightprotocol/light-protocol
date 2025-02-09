import { describe, it, expect } from 'vitest';
import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Program,
    AnchorProvider,
    setProvider,
    Wallet,
} from '@coral-xyz/anchor';
import {
    encodeInstructionDataInvoke,
    decodeInstructionDataInvoke,
    encodePublicTransactionEvent,
    decodePublicTransactionEvent,
    invokeAccountsLayout,
} from '../../src/programs/layout';
import { PublicTransactionEvent } from '../../src/state';

import { ACP_IDL, AccountCompression } from '../../src';
import { LightSystemProgram } from '../../src/programs/system';

const getTestProgram = (): Program<AccountCompression> => {
    const mockKeypair = Keypair.generate();
    const mockConnection = new Connection('http://127.0.0.1:8899', 'confirmed');
    const mockProvider = new AnchorProvider(
        mockConnection,
        new Wallet(mockKeypair),
        {
            commitment: 'confirmed',
        },
    );
    setProvider(mockProvider);
    return new Program(
        ACP_IDL,
        new PublicKey('compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq'),
        mockProvider,
    );
};

describe('acp', () => {
    it('should be able to create a new acp', async () => {
        const program = getTestProgram();
    });
});
