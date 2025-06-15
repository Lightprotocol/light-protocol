#!/usr/bin/env node

// Test script to verify the compiled version is correctly set
import { featureFlags } from './dist/cjs/node/index.cjs';

console.log('Testing build version...');
console.log('featureFlags.version:', featureFlags.version);
console.log('featureFlags.isV2():', featureFlags.isV2());

const expectedVersion = process.env.EXPECTED_VERSION || 'V1';
const actualVersion = featureFlags.version.replace(/['"]/g, '');
if (actualVersion === expectedVersion) {
    console.log(`✅ Success: Version is correctly set to ${expectedVersion}`);
    process.exit(0);
} else {
    console.error(
        `❌ Error: Expected version ${expectedVersion} but got ${actualVersion}`,
    );
    process.exit(1);
}
