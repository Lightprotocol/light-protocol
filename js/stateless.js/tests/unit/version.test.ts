import { describe, it, expect } from 'vitest';
import {
    featureFlags,
    VERSION,
    assertBetaEnabled,
    BETA_REQUIRED_ERROR,
} from '../../src/constants';

describe('Version System', () => {
    it('should have version set', () => {
        console.log('Current version:', featureFlags.version);
        console.log('isV2():', featureFlags.isV2());
        console.log(
            'Environment variable:',
            process.env.LIGHT_PROTOCOL_VERSION,
        );

        expect(featureFlags.version).toBeDefined();
        expect([VERSION.V1, VERSION.V2]).toContain(featureFlags.version);
    });

    it('should respect LIGHT_PROTOCOL_VERSION environment variable', () => {
        const expectedVersion =
            process.env.LIGHT_PROTOCOL_VERSION || VERSION.V1;
        expect(featureFlags.version).toBe(expectedVersion);
    });

    it('isV2() should return correct value', () => {
        const expectedIsV2 = featureFlags.version === VERSION.V2;
        expect(featureFlags.isV2()).toBe(expectedIsV2);
    });
});

describe('assertBetaEnabled', () => {
    it('should throw correct error based on version and beta flag', () => {
        const isV2 = featureFlags.isV2();
        const isBeta = featureFlags.isBeta();

        if (!isV2) {
            // V1 mode: should throw V2 required error
            expect(() => assertBetaEnabled()).toThrowError(
                'Interface methods require V2. Set LIGHT_PROTOCOL_VERSION=V2.',
            );
        } else if (!isBeta) {
            // V2 mode without beta: should throw beta required error
            expect(() => assertBetaEnabled()).toThrowError(BETA_REQUIRED_ERROR);
        } else {
            // V2 mode with beta: should not throw
            expect(() => assertBetaEnabled()).not.toThrow();
        }
    });

    it('V1 mode must reject interface methods with specific error message', () => {
        // This test ensures the V1 guard is in place and produces the expected error.
        // If running in V1 mode, this validates the error. If V2, it's a no-op check.
        if (!featureFlags.isV2()) {
            expect(() => assertBetaEnabled()).toThrowError(
                /Interface methods require V2/,
            );
        }
    });
});
