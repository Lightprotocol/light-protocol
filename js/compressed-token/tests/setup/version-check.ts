import { validateVersionConsistency } from '../../src/utils/version-check';

// Only used in tests.
export default function setup() {
    console.log('Checking version consistency...');

    try {
        validateVersionConsistency();
        const expectedVersion = process.env.LIGHT_PROTOCOL_VERSION || 'V1';
        console.log(`✅ Version check passed: Using ${expectedVersion}`);
    } catch (error) {
        console.error('❌ Version check failed:');
        console.error(error.message);
        process.exit(1);
    }
}
