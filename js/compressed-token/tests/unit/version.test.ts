import { describe, it, expect, beforeAll } from 'vitest';
import { featureFlags, VERSION } from '@lightprotocol/stateless.js';

describe('Versioning', () => {
    beforeAll(() => {
        const expectedVersion =
            process.env.LIGHT_PROTOCOL_VERSION || VERSION.V1;
        const actualVersion = featureFlags.version.replace(/['"]/g, '');

        if (actualVersion !== expectedVersion) {
            throw new Error(
                `Version mismatch detected!\n` +
                    `Expected: ${expectedVersion} (from LIGHT_PROTOCOL_VERSION env var)\n` +
                    `Actual: ${actualVersion} (from built stateless.js)\n\n` +
                    `This means stateless.js was built with ${actualVersion} but you're trying to test with ${expectedVersion}.\n` +
                    `Please rebuild stateless.js with the correct version:\n` +
                    `  cd ../stateless.js && pnpm build:${expectedVersion.toLowerCase()}\n` +
                    `Or use the compressed-token build command that handles this automatically:\n` +
                    `  pnpm build:${expectedVersion.toLowerCase()}`,
            );
        }
    });

    it('should use version from stateless.js', () => {
        console.log('Current version from stateless.js:', featureFlags.version);
        console.log('isV2() from stateless.js:', featureFlags.isV2());
        console.log(
            'Environment variable:',
            process.env.LIGHT_PROTOCOL_VERSION,
        );

        expect(featureFlags.version).toBeDefined();
        const actualVersion = featureFlags.version.replace(/['"]/g, '');
        expect([VERSION.V1, VERSION.V2]).toContain(actualVersion);
    });

    it('should respect LIGHT_PROTOCOL_VERSION environment variable', () => {
        const expectedVersion =
            process.env.LIGHT_PROTOCOL_VERSION || VERSION.V1;
        const actualVersion = featureFlags.version.replace(/['"]/g, '');
        expect(actualVersion).toBe(expectedVersion);
    });

    it('isV2() should return correct value', () => {
        const actualVersion = featureFlags.version.replace(/['"]/g, '');
        const expectedIsV2 = actualVersion === VERSION.V2;
        expect(featureFlags.isV2()).toBe(expectedIsV2);
    });

    it('compressed-token should use the same version as stateless.js', () => {
        const actualVersion = featureFlags.version.replace(/['"]/g, '');
        const isV2 = featureFlags.isV2();

        if (process.env.LIGHT_PROTOCOL_VERSION === 'V2') {
            expect(actualVersion).toBe(VERSION.V2);
            expect(isV2).toBe(true);
        } else {
            expect(actualVersion).toBe(VERSION.V1);
            expect(isV2).toBe(false);
        }
    });
});
