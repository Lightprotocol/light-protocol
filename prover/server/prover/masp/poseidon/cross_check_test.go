package poseidon

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// rustShimPath returns the path to the compiled Rust cross-check binary. If the
// binary does not exist, the caller should skip the test.
func rustShimPath(t *testing.T) string {
	_, thisFile, _, _ := runtime.Caller(0)
	p := filepath.Join(filepath.Dir(thisFile), "rust_cross_check", "target", "release", "hash_bn254")
	if _, err := os.Stat(p); err != nil {
		t.Skipf("rust shim not built at %s (run: cargo build --release --manifest-path poseidon/rust_cross_check/Cargo.toml)", p)
	}
	return p
}

// randomBE32 returns a random 32-byte BE value whose numerical interpretation
// is guaranteed to be < Modulus. Uses 31 random bytes + one leading zero byte
// (Modulus is ~254 bits, 31 bytes = 248 bits).
func randomBE32() ([]byte, error) {
	buf := make([]byte, 31)
	if _, err := rand.Read(buf); err != nil {
		return nil, err
	}
	out := make([]byte, 32)
	copy(out[1:], buf)
	return out, nil
}

func TestCrossCheckLightPoseidon(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping cross-check in -short")
	}
	shim := rustShimPath(t)

	const casesPerWidth = 10
	for width := MinWidth; width <= 13; width++ {
		width := width
		t.Run(fmt.Sprintf("t=%d", width), func(t *testing.T) {
			for c := 0; c < casesPerWidth; c++ {
				inputBytes := make([][]byte, width-1)
				inputHex := make([]string, width-1)
				for i := 0; i < width-1; i++ {
					be, err := randomBE32()
					if err != nil {
						t.Fatalf("rand: %v", err)
					}
					inputBytes[i] = be
					inputHex[i] = "0x" + hex.EncodeToString(be)
				}

				goBytes, err := HashBytesBE(inputBytes)
				if err != nil {
					t.Fatalf("Go hash: %v", err)
				}
				goHex := hex.EncodeToString(goBytes[:])

				args := append([]string{fmt.Sprintf("%d", width)}, inputHex...)
				out, err := exec.Command(shim, args...).Output()
				if err != nil {
					t.Fatalf("rust shim: %v", err)
				}
				rustHex := strings.TrimSpace(string(out))

				if goHex != rustHex {
					t.Fatalf("mismatch at t=%d case %d\n  inputs = %v\n  go   = %s\n  rust = %s",
						width, c, inputHex, goHex, rustHex)
				}
			}
		})
	}
}
