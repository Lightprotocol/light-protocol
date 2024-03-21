import { BN } from '@coral-xyz/anchor';
import { FIELD_SIZE } from '../constants';
import { CompressedProof_IdlType } from '../state';

interface GnarkProofJson {
    ar: string[];
    bs: string[][];
    krs: string[];
}

type ProofABC = {
    a: Uint8Array;
    b: Uint8Array;
    c: Uint8Array;
};

export function proofFromJsonStruct(json: GnarkProofJson): ProofABC {
    const proofAX = deserializeHexStringToBeBytes(json.ar[0]);
    const proofAY = deserializeHexStringToBeBytes(json.ar[1]);
    const proofA: Uint8Array = new Uint8Array([...proofAX, ...proofAY]);

    const proofBX0 = deserializeHexStringToBeBytes(json.bs[0][0]);
    const proofBX1 = deserializeHexStringToBeBytes(json.bs[0][1]);
    const proofBY0 = deserializeHexStringToBeBytes(json.bs[1][0]);
    const proofBY1 = deserializeHexStringToBeBytes(json.bs[1][1]);
    const proofB: Uint8Array = new Uint8Array([
        ...proofBX0,
        ...proofBX1,
        ...proofBY0,
        ...proofBY1,
    ]);

    const proofCX = deserializeHexStringToBeBytes(json.krs[0]);
    const proofCY = deserializeHexStringToBeBytes(json.krs[1]);
    const proofC: Uint8Array = new Uint8Array([...proofCX, ...proofCY]);

    const proofABC: ProofABC = { a: proofA, b: proofB, c: proofC };
    return proofABC;
}

// TODO: add unit test for negation
// TODO: test if LE BE issue. unit test
export function negateAndCompressProof(
    proof: ProofABC,
): CompressedProof_IdlType {
    const proofA = proof.a;
    const proofB = proof.b;
    const proofC = proof.c;

    const aXElement = proofA.slice(0, 32);
    const aYElement = new BN(proofA.slice(32, 64), 32, 'be');

    /// Negate
    const proofAIsPositive = yElementIsPositiveG1(aYElement) ? false : true;
    /// First byte of proofA is the bitmask
    aXElement[0] = addBitmaskToByte(aXElement[0], proofAIsPositive);

    const bXElement = proofB.slice(0, 64);
    const bYElement = proofB.slice(64, 128);

    const proofBIsPositive = yElementIsPositiveG2(
        new BN(bYElement.slice(0, 32), 32, 'be'),
        new BN(bYElement.slice(32, 64), 32, 'be'),
    );

    bXElement[0] = addBitmaskToByte(bXElement[0], proofBIsPositive);

    const cXElement = proofC.slice(0, 32);
    const cYElement = proofC.slice(32, 64);
    const proofCIsPositive = yElementIsPositiveG1(new BN(cYElement, 32, 'be'));
    cXElement[0] = addBitmaskToByte(cXElement[0], proofCIsPositive);

    const compressedProof: CompressedProof_IdlType = {
        a: Array.from(aXElement),
        b: Array.from(bXElement),
        c: Array.from(cXElement),
    };

    return compressedProof;
}

function deserializeHexStringToBeBytes(hexStr: string): Uint8Array {
    // Using BN for simpler conversion from hex string to byte array
    const bn = new BN(
        hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr,
        'hex',
    );
    return new Uint8Array(bn.toArray('be', 32));
}

function yElementIsPositiveG1(yElement: BN): boolean {
    return yElement.lte(FIELD_SIZE.sub(yElement));
}

function yElementIsPositiveG2(yElement1: BN, yElement2: BN): boolean {
    const fieldMidpoint = FIELD_SIZE.div(new BN(2));

    // Compare the first component of the y coordinate
    if (yElement1.lt(fieldMidpoint)) {
        return true;
    } else if (yElement1.gt(fieldMidpoint)) {
        return false;
    }

    // If the first component is equal to the midpoint, compare the second component
    return yElement2.lt(fieldMidpoint);
}
// bitmask compatible with solana altbn128 compression syscall and arkworks' implementation
// https://github.com/arkworks-rs/algebra/blob/master/ff/src/fields/models/fp/mod.rs#L580
// https://github.com/arkworks-rs/algebra/blob/master/serialize/src/flags.rs#L18
// fn u8_bitmask(value: u8, inf: bool, neg: bool) -> u8 {
//     let mut mask = 0;
//     match self {
//         inf => mask |= 1 << 6,
//         neg => mask |= 1 << 7,
//         _ => (),
//     }
//     mask
// }
function addBitmaskToByte(byte: number, yIsPositive: boolean): number {
    if (!yIsPositive) {
        return (byte |= 1 << 7);
    } else {
        return byte;
    }
}

if (import.meta.vitest) {
    const { it, expect, describe } = import.meta.vitest;

    // Unit test for addBitmaskToByte function
    describe('addBitmaskToByte', () => {
        it('should add a bitmask to the byte if yIsPositive is false', () => {
            const byte = 0b00000000;
            const yIsPositive = false;
            const result = addBitmaskToByte(byte, yIsPositive);
            expect(result).toBe(0b10000000); // 128 in binary, which is 1 << 7
        });

        it('should not modify the byte if yIsPositive is true', () => {
            const byte = 0b00000000;
            const yIsPositive = true;
            const result = addBitmaskToByte(byte, yIsPositive);
            expect(result).toBe(0b00000000);
        });
    });

    describe('test prover server', () => {
        const TEST_JSON = {
            ar: [
                '0x22bdaa3187d8fe294925a66fa0165a11bc9e07678fa2fc72402ebfd33d521c69',
                '0x2d18ff780b69898b4cdd8d7b6ac72d077799399f0f45e52665426456f3903584',
            ],
            bs: [
                [
                    '0x138cc0962e49f76a701d2871d2799892c9782940095eb0429e979f336d2e162d',
                    '0x2fe1bfbb15cbfb83d7e00ace23e45f890604003783eaf34affa35e0d6f4822bc',
                ],
                [
                    '0x1a89264f82cc6e8ef1c696bea0b5803c28c0ba6ab61366bcb71e73a4135cae8d',
                    '0xf778d857b3df01a4100265c9d014ce02d47425f0114685356165fa5ee3f3a26',
                ],
            ],
            krs: [
                '0x176b6ae9001f66832951e2d43a98a972667447bb1781f534b70cb010270dcdd3',
                '0xb748d5fac1686db28d94c02250af7eb4f28dfdabc8983305c45bcbc6e163eeb',
            ],
        };
        const COMPRESSED_PROOF_A = [
            34, 189, 170, 49, 135, 216, 254, 41, 73, 37, 166, 111, 160, 22, 90,
            17, 188, 158, 7, 103, 143, 162, 252, 114, 64, 46, 191, 211, 61, 82,
            28, 105,
        ];
        const COMPRESSED_PROOF_B = [
            147, 140, 192, 150, 46, 73, 247, 106, 112, 29, 40, 113, 210, 121,
            152, 146, 201, 120, 41, 64, 9, 94, 176, 66, 158, 151, 159, 51, 109,
            46, 22, 45, 47, 225, 191, 187, 21, 203, 251, 131, 215, 224, 10, 206,
            35, 228, 95, 137, 6, 4, 0, 55, 131, 234, 243, 74, 255, 163, 94, 13,
            111, 72, 34, 188,
        ];
        const COMPRESSED_PROOF_C = [
            23, 107, 106, 233, 0, 31, 102, 131, 41, 81, 226, 212, 58, 152, 169,
            114, 102, 116, 71, 187, 23, 129, 245, 52, 183, 12, 176, 16, 39, 13,
            205, 211,
        ];

        it('should execute a compressed token mint', async () => {
            const proof = proofFromJsonStruct(TEST_JSON);
            const compressedProof = negateAndCompressProof(proof);
            expect(compressedProof.a).toEqual(COMPRESSED_PROOF_A);
            expect(compressedProof.b).toEqual(COMPRESSED_PROOF_B);
            expect(compressedProof.c).toEqual(COMPRESSED_PROOF_C);
        });
    });
}
