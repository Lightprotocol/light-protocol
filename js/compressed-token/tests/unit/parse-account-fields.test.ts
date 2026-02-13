import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    convertTokenDataToAccount,
    parseCTokenHot,
} from '../../src/v3/get-account-interface';

/**
 * Build a 165-byte SPL-compatible token account buffer.
 *
 * Offsets (COption = 4-byte prefix):
 *   0-31   mint            (32)
 *  32-63   owner           (32)
 *  64-71   amount          (u64 LE)
 *  72-75   delegateOption  (u32 COption)
 *  76-107  delegate        (32)
 *  108     state           (u8: 0=Uninit, 1=Init, 2=Frozen)
 *  109-112 isNativeOption  (u32 COption)
 *  113-120 isNative        (u64 LE)
 *  121-128 delegatedAmount (u64 LE)
 *  129-132 closeAuthOption (u32 COption)
 *  133-164 closeAuthority  (32)
 */
function buildSplTokenBuffer(params: {
    mint: PublicKey;
    owner: PublicKey;
    amount: number;
    delegate?: PublicKey | null;
    state: number;
    isNative?: number | null;
    delegatedAmount?: number;
    closeAuthority?: PublicKey | null;
}): Buffer {
    const buf = Buffer.alloc(165);

    Buffer.from(params.mint.toBytes()).copy(buf, 0);
    Buffer.from(params.owner.toBytes()).copy(buf, 32);

    buf.writeUInt32LE(params.amount & 0xffffffff, 64);
    buf.writeUInt32LE(Math.floor(params.amount / 0x100000000), 68);

    if (params.delegate) {
        buf.writeUInt32LE(1, 72);
        Buffer.from(params.delegate.toBytes()).copy(buf, 76);
    }

    buf[108] = params.state;

    if (params.isNative != null) {
        buf.writeUInt32LE(1, 109);
        buf.writeUInt32LE(params.isNative & 0xffffffff, 113);
        buf.writeUInt32LE(Math.floor(params.isNative / 0x100000000), 117);
    }

    const da = params.delegatedAmount ?? 0;
    buf.writeUInt32LE(da & 0xffffffff, 121);
    buf.writeUInt32LE(Math.floor(da / 0x100000000), 125);

    if (params.closeAuthority) {
        buf.writeUInt32LE(1, 129);
        Buffer.from(params.closeAuthority.toBytes()).copy(buf, 133);
    }

    return buf;
}

/**
 * Build Borsh-serialized TLV Vec<ExtensionStruct> containing a single
 * CompressedOnly extension (discriminator 31).
 *
 * Format: [u32 vec_len] [u8 disc=31] [u64 delegated_amount] [u64 withheld_fee] [u8 is_ata]
 */
function buildCompressedOnlyTlv(
    delegatedAmount: number,
    withheldFee = 0,
    isAta = 0,
): Buffer {
    const buf = Buffer.alloc(4 + 1 + 17);
    buf.writeUInt32LE(1, 0);
    buf[4] = 31;
    buf.writeUInt32LE(delegatedAmount & 0xffffffff, 5);
    buf.writeUInt32LE(Math.floor(delegatedAmount / 0x100000000), 9);
    buf.writeUInt32LE(withheldFee & 0xffffffff, 13);
    buf.writeUInt32LE(Math.floor(withheldFee / 0x100000000), 17);
    buf[21] = isAta;
    return buf;
}

/**
 * Build TLV with multiple extensions before CompressedOnly.
 * Prepends `prefixDiscs` (each 0-byte unit variant), then CompressedOnly.
 */
function buildTlvWithPrefixExtensions(
    prefixDiscs: number[],
    delegatedAmount: number,
): Buffer {
    const vecLen = prefixDiscs.length + 1;
    const totalSize = 4 + prefixDiscs.length + 1 + 17;
    const buf = Buffer.alloc(totalSize);
    let offset = 0;

    buf.writeUInt32LE(vecLen, offset);
    offset += 4;

    for (const disc of prefixDiscs) {
        buf[offset] = disc;
        offset += 1;
    }

    buf[offset] = 31;
    offset += 1;
    buf.writeUInt32LE(delegatedAmount & 0xffffffff, offset);
    buf.writeUInt32LE(Math.floor(delegatedAmount / 0x100000000), offset + 4);
    return buf;
}

describe('parseCTokenHot - COption format correctness', () => {
    it('should parse initialized state at offset 108 (regression: old parser read offset 105)', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const address = Keypair.generate().publicKey;

        const data = buildSplTokenBuffer({
            mint,
            owner,
            amount: 1000,
            state: 1,
        });

        const result = parseCTokenHot(address, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.isInitialized).toBe(true);
        expect(result.parsed.isFrozen).toBe(false);
        expect(result.parsed.amount).toBe(1000n);
        expect(result.parsed.delegate).toBeNull();
        expect(result.parsed.delegatedAmount).toBe(0n);
        expect(result.parsed.isNative).toBe(false);
        expect(result.parsed.closeAuthority).toBeNull();
        expect(result.isCold).toBe(false);
    });

    it('should parse frozen state correctly', () => {
        const data = buildSplTokenBuffer({
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: 500,
            state: 2,
        });

        const result = parseCTokenHot(Keypair.generate().publicKey, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.isInitialized).toBe(true);
        expect(result.parsed.isFrozen).toBe(true);
    });

    it('should parse delegate and delegatedAmount from COption layout', () => {
        const delegate = Keypair.generate().publicKey;
        const data = buildSplTokenBuffer({
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: 5000,
            delegate,
            state: 1,
            delegatedAmount: 3000,
        });

        const result = parseCTokenHot(Keypair.generate().publicKey, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.delegate!.toBase58()).toBe(delegate.toBase58());
        expect(result.parsed.delegatedAmount).toBe(3000n);
        expect(result.parsed.amount).toBe(5000n);
        expect(result.parsed.isInitialized).toBe(true);
    });

    it('should parse closeAuthority from COption at offset 129', () => {
        const closeAuth = Keypair.generate().publicKey;
        const data = buildSplTokenBuffer({
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: 100,
            state: 1,
            closeAuthority: closeAuth,
        });

        const result = parseCTokenHot(Keypair.generate().publicKey, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.closeAuthority!.toBase58()).toBe(
            closeAuth.toBase58(),
        );
    });

    it('should parse isNative correctly when set', () => {
        const data = buildSplTokenBuffer({
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: 100,
            state: 1,
            isNative: 2039280,
        });

        const result = parseCTokenHot(Keypair.generate().publicKey, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.isNative).toBe(true);
        expect(result.parsed.rentExemptReserve).toBe(2039280n);
    });

    it('should parse mint and owner pubkeys correctly', () => {
        const mint = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const address = Keypair.generate().publicKey;
        const data = buildSplTokenBuffer({ mint, owner, amount: 0, state: 1 });

        const result = parseCTokenHot(address, {
            executable: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: 1_000_000,
            data,
            rentEpoch: undefined,
        });

        expect(result.parsed.mint.toBase58()).toBe(mint.toBase58());
        expect(result.parsed.owner.toBase58()).toBe(owner.toBase58());
        expect(result.parsed.address.toBase58()).toBe(address.toBase58());
    });
});

describe('convertTokenDataToAccount - delegatedAmount logic', () => {
    it('should return 0 when no delegate and no TLV', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(1000),
            delegate: null,
            state: 1,
            tlv: null,
        });
        expect(result.delegatedAmount).toBe(0n);
    });

    it('should equal amount when delegate is set and no TLV (compressed approve)', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(5000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: null,
        });
        expect(result.delegatedAmount).toBe(5000n);
    });

    it('should extract delegatedAmount from CompressedOnly extension in TLV', () => {
        const tlv = buildCompressedOnlyTlv(3000);

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(10000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv,
        });
        expect(result.delegatedAmount).toBe(3000n);
    });

    it('should return 0 from CompressedOnly when delegated_amount is 0 (no delegate on source)', () => {
        const tlv = buildCompressedOnlyTlv(0);

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(10000),
            delegate: null,
            state: 1,
            tlv,
        });
        expect(result.delegatedAmount).toBe(0n);
    });

    it('should find CompressedOnly after fixed-size extensions (PausableAccount + PermanentDelegate)', () => {
        const tlv = buildTlvWithPrefixExtensions([27, 28], 7777);

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(10000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv,
        });
        expect(result.delegatedAmount).toBe(7777n);
    });

    it('should find CompressedOnly after TransferFeeAccount (8-byte extension)', () => {
        // TLV: vec_len=2, [disc=29, 8 bytes fee data], [disc=31, 17 bytes CompressedOnly]
        const buf = Buffer.alloc(4 + 1 + 8 + 1 + 17);
        buf.writeUInt32LE(2, 0);
        buf[4] = 29; // TransferFeeAccountExtension
        buf.writeUInt32LE(999, 5); // withheld_amount (8 bytes, only lo)
        buf[13] = 31; // CompressedOnly
        buf.writeUInt32LE(4200, 14); // delegated_amount lo

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(10000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: buf,
        });
        expect(result.delegatedAmount).toBe(4200n);
    });

    it('should find CompressedOnly after TransferHookAccount (1-byte extension)', () => {
        // TLV: vec_len=2, [disc=30, 1 byte], [disc=31, 17 bytes]
        const buf = Buffer.alloc(4 + 1 + 1 + 1 + 17);
        buf.writeUInt32LE(2, 0);
        buf[4] = 30; // TransferHookAccountExtension
        buf[5] = 0; // transferring = false
        buf[6] = 31; // CompressedOnly
        buf.writeUInt32LE(1234, 7);

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(10000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: buf,
        });
        expect(result.delegatedAmount).toBe(1234n);
    });

    it('should fall back to amount when variable-length extension blocks TLV parsing', () => {
        // TokenMetadata (disc 19) is variable-length; can't skip past it
        const buf = Buffer.alloc(4 + 1 + 50);
        buf.writeUInt32LE(2, 0);
        buf[4] = 19; // TokenMetadata (variable)
        // CompressedOnly follows but is unreachable

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(8000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: buf,
        });
        // Falls back: delegate set + TLV unparseable => delegatedAmount = amount
        expect(result.delegatedAmount).toBe(8000n);
    });

    it('should return 0 when TLV has no CompressedOnly and no delegate', () => {
        // TLV with only PausableAccount
        const buf = Buffer.alloc(4 + 1);
        buf.writeUInt32LE(1, 0);
        buf[4] = 27; // PausableAccount (0 bytes)

        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(5000),
            delegate: null,
            state: 1,
            tlv: buf,
        });
        expect(result.delegatedAmount).toBe(0n);
    });

    it('should handle empty TLV buffer gracefully', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(1000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: Buffer.alloc(0),
        });
        // Empty TLV can't be parsed => falls back to amount
        expect(result.delegatedAmount).toBe(1000n);
    });

    it('should handle truncated TLV buffer gracefully', () => {
        // 3 bytes: too short for even vec_len (4 bytes)
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(2000),
            delegate: Keypair.generate().publicKey,
            state: 1,
            tlv: Buffer.alloc(3),
        });
        expect(result.delegatedAmount).toBe(2000n);
    });
});

describe('convertTokenDataToAccount - other parsed fields', () => {
    it('should set isInitialized=true for state=1', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 1,
            tlv: null,
        });
        expect(result.isInitialized).toBe(true);
        expect(result.isFrozen).toBe(false);
    });

    it('should set isFrozen=true for state=2', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 2,
            tlv: null,
        });
        expect(result.isInitialized).toBe(true);
        expect(result.isFrozen).toBe(true);
    });

    it('should set isInitialized=false for state=0', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 0,
            tlv: null,
        });
        expect(result.isInitialized).toBe(false);
        expect(result.isFrozen).toBe(false);
    });

    it('should hardcode isNative=false and closeAuthority=null for cold accounts', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 1,
            tlv: null,
        });
        expect(result.isNative).toBe(false);
        expect(result.rentExemptReserve).toBeNull();
        expect(result.closeAuthority).toBeNull();
    });

    it('should pass through TLV data as tlvData buffer', () => {
        const tlv = Buffer.from([1, 2, 3, 4, 5]);
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 1,
            tlv,
        });
        expect(result.tlvData).toEqual(Buffer.from([1, 2, 3, 4, 5]));
    });

    it('should return empty tlvData when tlv is null', () => {
        const result = convertTokenDataToAccount(Keypair.generate().publicKey, {
            mint: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            amount: new BN(100),
            delegate: null,
            state: 1,
            tlv: null,
        });
        expect(result.tlvData).toEqual(Buffer.alloc(0));
    });
});
