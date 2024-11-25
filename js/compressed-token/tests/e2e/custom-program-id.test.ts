import { describe, it, expect } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { PublicKey } from '@solana/web3.js';

describe('custom programId', () => {
    it('should switch programId', async () => {
        const defaultProgramId = new PublicKey(
            'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
        );
        const solMint = new PublicKey(
            'So11111111111111111111111111111111111111112',
        );
        const expectedPoolPda = new PublicKey(
            '3EJpXEsHL6JxNoPJWjF4QTKuvFvxzsUPbf1xF8iMbnL7',
        );
        const newProgramId = new PublicKey(
            '2WpGefPmpKMbkyLewupcfb8DuJ1ZMSPkMSu5WEvDMpF4',
        );

        // Check default program ID
        expect(CompressedTokenProgram.programId).toEqual(defaultProgramId);
        expect(CompressedTokenProgram.deriveTokenPoolPda(solMint)).toEqual(
            expectedPoolPda,
        );
        expect(CompressedTokenProgram.program.programId).toEqual(
            defaultProgramId,
        );

        // Set new program ID
        CompressedTokenProgram.setProgramId(newProgramId);

        // Verify program ID was updated
        expect(CompressedTokenProgram.programId).toEqual(newProgramId);
        expect(CompressedTokenProgram.deriveTokenPoolPda(solMint)).not.toEqual(
            expectedPoolPda,
        );
        expect(CompressedTokenProgram.program.programId).toEqual(newProgramId);

        // Reset program ID
        CompressedTokenProgram.setProgramId(defaultProgramId);
    });
});
