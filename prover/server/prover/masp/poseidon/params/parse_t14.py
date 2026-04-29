#!/usr/bin/env python3
"""Parse SageMath generate_parameters_grain.sage output for Poseidon t=14
and emit a Go constants file.

Input:  /Users/ananas/dev/experiments/shielded-pool-bench/poseidon/params/t14_raw.txt
Output: /Users/ananas/dev/experiments/shielded-pool-bench/poseidon/constants_bn254_x5_t14.go
"""

import ast
import sys

INPUT_PATH = "/Users/ananas/dev/experiments/shielded-pool-bench/poseidon/params/t14_raw.txt"
OUTPUT_PATH = "/Users/ananas/dev/experiments/shielded-pool-bench/poseidon/constants_bn254_x5_t14.go"

BN254_MODULUS = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001

EXPECTED_ARK_LEN = 1092
EXPECTED_T = 14


def normalize_hex(x: str) -> str:
    """Normalize a hex string to exactly 0x + 64 nibbles (lowercase)."""
    assert isinstance(x, str), f"expected str, got {type(x)}"
    assert x.startswith("0x") or x.startswith("0X"), f"missing 0x prefix: {x!r}"
    v = int(x, 16)
    assert 0 <= v < BN254_MODULUS, (
        f"value out of range (>= BN254 modulus): {x}"
    )
    return "0x" + format(v, "064x")


def main() -> int:
    with open(INPUT_PATH, "r") as f:
        lines = f.read().splitlines()

    # Actual 0-indexed layout (no blank line after ARK):
    #  0: "Number of round constants: 1092"
    #  1: "Round constants for GF(p):"
    #  2: Python list literal of round constants
    #  3: "n: 254"
    #  4: "t: 14"
    #  5: "N: 3556"
    #  6-11: Algorithm results
    #  12: "Prime number: 0x..."
    #  13: "MDS matrix:"
    #  14: Python nested list — MDS matrix
    assert lines[0].startswith("Number of round constants:"), f"unexpected line 0: {lines[0]!r}"
    assert lines[1].startswith("Round constants for GF(p):"), f"unexpected line 1: {lines[1]!r}"
    assert lines[3].startswith("n:"), f"unexpected line 3: {lines[3]!r}"
    assert lines[4].startswith("t:"), f"unexpected line 4: {lines[4]!r}"
    assert lines[4].split(":", 1)[1].strip() == str(EXPECTED_T), (
        f"unexpected t value: {lines[4]!r}"
    )
    assert lines[13].startswith("MDS matrix:"), f"unexpected line 13: {lines[13]!r}"

    ark = ast.literal_eval(lines[2].strip())
    mds = ast.literal_eval(lines[14].strip())

    # Validate ARK
    assert isinstance(ark, list), f"ARK is not a list: {type(ark)}"
    assert len(ark) == EXPECTED_ARK_LEN, (
        f"ARK length {len(ark)} != expected {EXPECTED_ARK_LEN}"
    )
    ark_norm = [normalize_hex(x) for x in ark]

    # Validate MDS
    assert isinstance(mds, list), f"MDS is not a list: {type(mds)}"
    assert len(mds) == EXPECTED_T, f"MDS row count {len(mds)} != {EXPECTED_T}"
    mds_norm = []
    for i, row in enumerate(mds):
        assert isinstance(row, list), f"MDS row {i} is not a list"
        assert len(row) == EXPECTED_T, (
            f"MDS row {i} has {len(row)} columns, expected {EXPECTED_T}"
        )
        mds_norm.append([normalize_hex(x) for x in row])

    # Emit Go file
    out = []
    out.append(
        "// Code generated from hadeshash sage script for Poseidon t=14 "
        "(BN254, RF=8, RP=70). DO NOT EDIT."
    )
    out.append(
        "// Command: sage generate_parameters_grain.sage 1 0 254 14 8 70 "
        "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"
    )
    out.append("package poseidon")
    out.append("")
    out.append("import \"math/big\"")
    out.append("")
    out.append(
        "// ARK_14: flat round-constant vector, length (8+70)*14 = 1092"
    )
    out.append("var ARK_14 = []*big.Int{")
    for v in ark_norm:
        out.append(f"\th(\"{v}\"),")
    out.append("}")
    out.append("")
    out.append("// MDS_14: 14x14 MDS matrix")
    out.append("var MDS_14 = [][]*big.Int{")
    for row in mds_norm:
        out.append("\t{")
        for v in row:
            out.append(f"\t\th(\"{v}\"),")
        out.append("\t},")
    out.append("}")
    out.append("")

    content = "\n".join(out)

    with open(OUTPUT_PATH, "w") as f:
        f.write(content)

    print(f"Wrote {OUTPUT_PATH}")
    print(f"  ARK_14 length: {len(ark_norm)}")
    print(f"  MDS_14 shape:  {len(mds_norm)}x{len(mds_norm[0])}")
    print(f"  ARK_14[0]:     {ark_norm[0]}")
    print(f"  MDS_14[0][0]:  {mds_norm[0][0]}")
    print(f"  MDS_14[13][13]:{mds_norm[13][13]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
