import { describe, it, expect } from 'vitest';
import { MAX_TOP_UP } from '../../src/constants';

describe('constants', () => {
    describe('MAX_TOP_UP', () => {
        it('should equal 65535 (u16::MAX, no cap)', () => {
            expect(MAX_TOP_UP).toBe(65535);
        });
    });
});
