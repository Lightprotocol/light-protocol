package main

// Writes the per-circuit artifacts our rust-gnark Android/iOS prover loads:
//   <base>.r1cs, <base>.pk, <base>.vk, <base>.witness.json
//
// Format matches `light-mopro/prover/server/cmd/export-to-gnark` so the
// same mmap loader + Bench screen consumes both Light Protocol circuits
// and the zkp-bench ones without special casing.

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/consensys/gnark-crypto/ecc"
	fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	"github.com/consensys/gnark/backend/groth16"
	cs_bn254 "github.com/consensys/gnark/constraint/bn254"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
)

func exportCircuit(dir, base string, circuit, assignment frontend.Circuit) error {
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit)
	if err != nil {
		return fmt.Errorf("compile %s: %w", base, err)
	}

	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		return fmt.Errorf("setup %s: %w", base, err)
	}

	writeStd := func(path string, w func(io.Writer) (int64, error)) error {
		f, err := os.Create(path)
		if err != nil {
			return err
		}
		defer f.Close()
		_, err = w(f)
		return err
	}
	if err := writeStd(filepath.Join(dir, base+".r1cs"), func(w io.Writer) (int64, error) {
		return ccs.WriteTo(w)
	}); err != nil {
		return err
	}
	if err := writeStd(filepath.Join(dir, base+".pk"), func(w io.Writer) (int64, error) {
		return pk.WriteTo(w)
	}); err != nil {
		return err
	}
	if err := writeStd(filepath.Join(dir, base+".vk"), func(w io.Writer) (int64, error) {
		return vk.WriteTo(w)
	}); err != nil {
		return err
	}

	rcs, ok := ccs.(*cs_bn254.R1CS)
	if !ok {
		return fmt.Errorf("expected *cs_bn254.R1CS, got %T", ccs)
	}
	publicNames := rcs.Public
	if len(publicNames) > 0 && publicNames[0] == "1" {
		publicNames = publicNames[1:]
	}
	secretNames := rcs.Secret

	w, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		return fmt.Errorf("witness %s: %w", base, err)
	}
	vec, ok := w.Vector().(fr.Vector)
	if !ok {
		return fmt.Errorf("unexpected witness vector type for %s: %T", base, w.Vector())
	}
	if len(vec) != len(publicNames)+len(secretNames) {
		return fmt.Errorf("witness length %d != public %d + secret %d",
			len(vec), len(publicNames), len(secretNames))
	}

	jsonMap := make(map[string]string, len(vec))
	for i, name := range publicNames {
		jsonMap[name] = vec[i].String()
	}
	for i, name := range secretNames {
		jsonMap[name] = vec[len(publicNames)+i].String()
	}
	b, err := json.MarshalIndent(jsonMap, "", "  ")
	if err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(dir, base+".witness.json"), b, 0o644); err != nil {
		return err
	}
	fmt.Printf("wrote %s (%d constraints) to %s\n", base, ccs.GetNbConstraints(), dir)
	return nil
}
