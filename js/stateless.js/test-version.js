#!/usr/bin/env node

// Test script to verify the compiled version is correctly set
const { featureFlags } = require('./dist/cjs/node/index.cjs');

console.log('Testing build version...');
console.log('featureFlags.version:', featureFlags.version);
console.log('featureFlags.isV2():', featureFlags.isV2());

const expectedVersion = process.env.EXPECTED_VERSION || 'V1';
if (featureFlags.version === expectedVersion) {
    console.log(`✅ Success: Version is correctly set to ${expectedVersion}`);
    process.exit(0);
} else {
    console.error(
        `❌ Error: Expected version ${expectedVersion} but got ${featureFlags.version}`,
    );
    process.exit(1);
}
