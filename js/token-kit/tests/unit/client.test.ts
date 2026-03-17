/**
 * Unit tests for client-level shared error and validation types.
 *
 * Selection and load helper behavior is covered in selection.test.ts and load.test.ts.
 */

import { describe, it, expect } from 'vitest';

import {
    assertV2Tree,
    TreeType,
    IndexerError,
    IndexerErrorCode,
} from '../../src/index.js';

describe('IndexerError', () => {
    it('constructs with code, message, and cause', () => {
        const cause = new Error('Original error');
        const error = new IndexerError(
            IndexerErrorCode.NetworkError,
            'Connection failed',
            cause,
        );

        expect(error.code).toBe(IndexerErrorCode.NetworkError);
        expect(error.message).toBe('Connection failed');
        expect(error.cause).toBe(cause);
        expect(error.name).toBe('IndexerError');
        expect(error instanceof Error).toBe(true);
    });

    it('supports construction without cause', () => {
        const error = new IndexerError(
            IndexerErrorCode.InvalidResponse,
            'Bad response',
        );

        expect(error.code).toBe(IndexerErrorCode.InvalidResponse);
        expect(error.message).toBe('Bad response');
        expect(error.cause).toBeUndefined();
    });
});

describe('assertV2Tree', () => {
    it('accepts all known tree types', () => {
        expect(() => assertV2Tree(TreeType.StateV1)).not.toThrow();
        expect(() => assertV2Tree(TreeType.AddressV1)).not.toThrow();
        expect(() => assertV2Tree(TreeType.StateV2)).not.toThrow();
        expect(() => assertV2Tree(TreeType.AddressV2)).not.toThrow();
    });

    it('throws for unknown tree types', () => {
        expect(() => assertV2Tree(99 as TreeType)).toThrow(IndexerError);
        expect(() => assertV2Tree(99 as TreeType)).toThrow('Unknown tree type');
    });
});

