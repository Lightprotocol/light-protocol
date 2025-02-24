import { describe, it, expect } from 'vitest';
import { validateSufficientBalance } from '../../../src/utils/validation';
import { bn } from '../../../src/state';
import {
    validateNumbersForProof,
    validateNumbersForInclusionProof,
    validateNumbersForNonInclusionProof,
    validateNumbers,
} from '../../../src/utils/validation';

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

describe('validateNumbersForProof', () => {
    it('should throw error for invalid hashesLength of 8 with non-zero newAddressesLength', () => {
        expect(() => validateNumbersForProof(8, 1)).toThrow();
    });

    it('should not throw error for valid hashesLength and newAddressesLength', () => {
        expect(() => validateNumbersForProof(2, 1)).not.toThrow();
    });

    it('should throw error for invalid hashesLength with zero newAddressesLength', () => {
        expect(() => validateNumbersForProof(5, 0)).toThrow();
    });

    it('should throw error for invalid newAddressesLength with zero hashesLength', () => {
        expect(() => validateNumbersForProof(0, 3)).toThrow();
    });

    it('should throw error for invalid hashesLength with non-zero newAddressesLength', () => {
        expect(() => validateNumbersForProof(8, 1)).toThrow();
    });
});

describe('validateNumbersForInclusionProof', () => {
    it('should not throw error for valid hashesLength', () => {
        expect(() => validateNumbersForInclusionProof(4)).not.toThrow();
    });

    it('should throw error for invalid hashesLength', () => {
        expect(() => validateNumbersForInclusionProof(5)).toThrow();
    });
});

describe('validateNumbersForNonInclusionProof', () => {
    it('should not throw error for valid newAddressesLength', () => {
        expect(() => validateNumbersForNonInclusionProof(1)).not.toThrow();
    });
    it('should not throw error for valid newAddressesLength', () => {
        expect(() => validateNumbersForNonInclusionProof(2)).not.toThrow();
    });

    it('should throw error for invalid newAddressesLength', () => {
        expect(() => validateNumbersForNonInclusionProof(3)).toThrow();
    });
});

describe('validateNumbers', () => {
    it('should not throw error for valid length', () => {
        expect(() => validateNumbers(2, [1, 2, 3], 'test type')).not.toThrow();
    });

    it('should throw error for invalid length', () => {
        expect(() => validateNumbers(4, [1, 2, 3], 'test type')).toThrow();
    });
});

describe('validateNumbersForProof', () => {
    it('should not throw error for valid hashesLength and newAddressesLength', () => {
        expect(() => validateNumbersForProof(2, 1)).not.toThrow();
    });

    it('should throw error for invalid hashesLength with zero newAddressesLength', () => {
        expect(() => validateNumbersForProof(5, 0)).toThrowError(
            'Invalid number of compressed accounts: 5. Allowed numbers: 1, 2, 3, 4, 8',
        );
    });

    it('should throw error for invalid newAddressesLength with zero hashesLength', () => {
        expect(() => validateNumbersForProof(0, 3)).toThrowError(
            'Invalid number of new addresses: 3. Allowed numbers: 1, 2',
        );
    });

    it('should throw error for invalid hashesLength with non-zero newAddressesLength', () => {
        expect(() => validateNumbersForProof(8, 1)).toThrowError(
            'Invalid number of compressed accounts for proof: 8. Allowed numbers: 1, 2, 3, 4',
        );
    });
});
