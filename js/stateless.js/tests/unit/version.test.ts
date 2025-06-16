import { describe, it, expect } from 'vitest';
import { featureFlags, VERSION } from '../../src/constants';

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
