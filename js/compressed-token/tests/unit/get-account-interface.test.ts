import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { getAccountInterface } from '../../src/mint/get-account-interface';
import { createRpc, newAccountWithLamports } from '@lightprotocol/stateless.js';

describe('getAccountInterface - Auto-Detection', () => {
    it('should have auto-detection signature without programId default', () => {
        // Type-level test: ensure programId is optional
        const testCall = async () => {
            const rpc = createRpc();
            const address = Keypair.generate().publicKey;

            // This should compile - programId is optional
            try {
                await getAccountInterface(rpc, address);
            } catch (e) {
                // Expected to fail since account doesn't exist
                expect(e).toBeDefined();
            }
        };

        expect(testCall).toBeDefined();
    });

    it('should fail with clear error message when account not found', async () => {
        const rpc = createRpc();
        const fakeAddress = Keypair.generate().publicKey;

        await expect(getAccountInterface(rpc, fakeAddress)).rejects.toThrow(
            'Token account not found',
        );
    });
});
