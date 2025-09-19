package common

import (
	"encoding/hex"
	"fmt"
	"math/big"
	"strings"
)

func ParseHexStringList(input string) ([]big.Int, error) {
	hexStrings := strings.Split(input, ",")
	result := make([]big.Int, len(hexStrings))

	for i, hexString := range hexStrings {
		hexString = strings.TrimSpace(hexString)
		hexString = strings.TrimPrefix(hexString, "0x")

		bytes, err := hex.DecodeString(hexString)
		if err != nil {
			return nil, fmt.Errorf("invalid hex string: %s", hexString)
		}

		result[i].SetBytes(bytes)
	}

	return result, nil
}

func ParseBigInt(input string) (*big.Int, error) {
	input = strings.TrimSpace(input)
	input = strings.TrimPrefix(input, "0x")

	bytes, err := hex.DecodeString(input)
	if err != nil {
		return nil, fmt.Errorf("invalid hex string: %s", input)
	}

	bigInt := new(big.Int).SetBytes(bytes)
	return bigInt, nil
}
