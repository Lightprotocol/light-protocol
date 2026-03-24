import { describe, it, expect } from 'vitest';
import {
    featureFlags,
    VERSION,
    assertV2Enabled,
    V2_REQUIRED_ERROR,
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
        // Default is V2 when no env var is set (see constants.ts line 31)
        const expectedVersion =
            process.env.LIGHT_PROTOCOL_VERSION || VERSION.V2;
        expect(featureFlags.version).toBe(expectedVersion);
    });

    it('isV2() should return correct value', () => {
        const expectedIsV2 = featureFlags.version === VERSION.V2;
        expect(featureFlags.isV2()).toBe(expectedIsV2);
    });
});

describe('assertV2Enabled', () => {
    it('should throw only when V2 is disabled', () => {
        if (!featureFlags.isV2()) {
            expect(() => assertV2Enabled()).toThrowError(V2_REQUIRED_ERROR);
        } else {
            expect(() => assertV2Enabled()).not.toThrow();
        }
    });

    it('V1 mode must reject interface methods with specific error message', () => {
        // This test ensures the V1 guard is in place and produces the expected error.
        // If running in V1 mode, this validates the error. If V2, it's a no-op check.
        if (!featureFlags.isV2()) {
            expect(() => assertV2Enabled()).toThrowError(
                /Interface methods require V2/,
            );
        }
    });
});
