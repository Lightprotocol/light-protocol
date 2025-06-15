import { featureFlags, VERSION } from '@lightprotocol/stateless.js';

/**
 * Validates that the built version of stateless.js matches the expected version.
 * Throws an error if there's a mismatch.
 *
 * @param expectedVersion - The version expected (defaults to LIGHT_PROTOCOL_VERSION env var or V1)
 * @throws Error if the versions don't match
 */
export function validateVersionConsistency(expectedVersion?: string): void {
    const expected =
        expectedVersion || process.env.LIGHT_PROTOCOL_VERSION || VERSION.V1;
    const actual = featureFlags.version.replace(/['"]/g, '');

    if (actual !== expected) {
        throw new Error(
            `Version mismatch detected!\n` +
                `Expected: ${expected} (from ${expectedVersion ? 'parameter' : 'LIGHT_PROTOCOL_VERSION env var'})\n` +
                `Actual: ${actual} (from built stateless.js)\n\n` +
                `This means stateless.js was built with ${actual} but you're trying to use ${expected}.\n` +
                `Please rebuild both packages with the same version:\n` +
                `  pnpm build:${expected.toLowerCase()}\n` +
                `This command will automatically build both stateless.js and compressed-token with ${expected}.`,
        );
    }
}

/**
 * Gets the current version from stateless.js
 */
export function getCurrentVersion(): string {
    return featureFlags.version.replace(/['"]/g, '');
}

/**
 * Checks if the current version is V2
 */
export function isV2(): boolean {
    return featureFlags.isV2();
}
