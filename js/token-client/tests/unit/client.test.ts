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
} from '@lightprotocol/token-sdk';

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
    it('throws for StateV1 tree type', () => {
        expect(() => assertV2Tree(TreeType.StateV1)).toThrow(IndexerError);
        expect(() => assertV2Tree(TreeType.StateV1)).toThrow(
            'V1 tree types are not supported',
        );
    });

    it('throws for AddressV1 tree type', () => {
        expect(() => assertV2Tree(TreeType.AddressV1)).toThrow(IndexerError);
        expect(() => assertV2Tree(TreeType.AddressV1)).toThrow(
            'V1 tree types are not supported',
        );
    });

    it('passes for V2 tree types', () => {
        expect(() => assertV2Tree(TreeType.StateV2)).not.toThrow();
        expect(() => assertV2Tree(TreeType.AddressV2)).not.toThrow();
    });

    it('throws InvalidResponse error code for V1 trees', () => {
        try {
            assertV2Tree(TreeType.StateV1);
            expect.fail('Expected assertV2Tree to throw');
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InvalidResponse,
            );
        }
    });
});

