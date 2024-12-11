import { describe, it, expect } from 'vitest';
import { validateSufficientBalance } from '../../../src/utils/validation';
import { bn } from '../../../src/state';

describe('validateSufficientBalance', () => {
    it('should not throw error for positive balance', () => {
        expect(() => validateSufficientBalance(bn(100))).not.toThrow();
    });

    it('should not throw error for zero balance', () => {
        expect(() => validateSufficientBalance(bn(0))).not.toThrow();
    });

    it('should throw error for negative balance', () => {
        expect(() => validateSufficientBalance(bn(-1))).toThrow();
    });
});
