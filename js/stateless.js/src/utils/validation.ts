import BN from 'bn.js';
import {
    CompressedAccountLegacy,
    CompressedAccountWithMerkleContext,
    bn,
} from '../state';

export const validateSufficientBalance = (balance: BN) => {
    if (balance.lt(bn(0))) {
        throw new Error('Insufficient balance for transfer');
    }
};

export const validateSameOwner = (
    compressedAccounts:
        | CompressedAccountLegacy[]
        | CompressedAccountWithMerkleContext[],
) => {
    if (compressedAccounts.length === 0) {
        throw new Error('No accounts provided for validation');
    }
    const zerothOwner = compressedAccounts[0].owner;
    if (
        !compressedAccounts.every(account => account.owner.equals(zerothOwner))
    ) {
        throw new Error('All input accounts must have the same owner');
    }
};

/// for V1 circuits.
export const validateNumbersForProof = (
    hashesLength: number,
    newAddressesLength: number,
) => {
    if (hashesLength > 0 && newAddressesLength > 0) {
        if (hashesLength === 8) {
            throw new Error(
                `Invalid number of compressed accounts for proof: ${hashesLength}. Allowed numbers: ${[1, 2, 3, 4].join(', ')}`,
            );
        }
        validateNumbers(hashesLength, [1, 2, 3, 4], 'compressed accounts');
        validateNumbersForNonInclusionProof(newAddressesLength);
    } else {
        if (hashesLength > 0) {
            validateNumbersForInclusionProof(hashesLength);
        } else {
            validateNumbersForNonInclusionProof(newAddressesLength);
        }
    }
};

/// Ensure that the amount if compressed accounts is allowed.
export const validateNumbersForInclusionProof = (hashesLength: number) => {
    validateNumbers(hashesLength, [1, 2, 3, 4, 8], 'compressed accounts');
};

/// Ensure that the amount if new addresses is allowed.
export const validateNumbersForNonInclusionProof = (
    newAddressesLength: number,
) => {
    validateNumbers(newAddressesLength, [1, 2], 'new addresses');
};

/// V1 circuit safeguards.
export const validateNumbers = (
    length: number,
    allowedNumbers: number[],
    type: string,
) => {
    if (!allowedNumbers.includes(length)) {
        throw new Error(
            `Invalid number of ${type}: ${length}. Allowed numbers: ${allowedNumbers.join(', ')}`,
        );
    }
};
