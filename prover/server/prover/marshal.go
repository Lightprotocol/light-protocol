package prover

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"math/big"
	"os"
	"strings"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
)

func fromHex(i *big.Int, s string) error {
	s = strings.TrimPrefix(s, "0x")
	_, ok := i.SetString(s, 16)
	if !ok {
		return fmt.Errorf("invalid number: %s", s)
	}
	return nil
}

func toHex(i *big.Int) string {
	return fmt.Sprintf("0x%064x", i)
}

func fromDec(i *big.Int, s string) error {
	_, ok := i.SetString(s, 10)
	if !ok {
		return fmt.Errorf("invalid number: %s", s)
	}
	return nil
}

func toDec(i *big.Int) string {
	return fmt.Sprintf("%s", i.Text(10))
}

type ProofJSON struct {
	Ar  [2]string    `json:"ar"`
	Bs  [2][2]string `json:"bs"`
	Krs [2]string    `json:"krs"`
}

func (p *Proof) MarshalJSON() ([]byte, error) {
	const fpSize = 32
	var buf bytes.Buffer
	_, err := p.Proof.WriteRawTo(&buf)
	if err != nil {
		return nil, err
	}
	proofBytes := buf.Bytes()
	proofJson := ProofJSON{}
	proofHexNumbers := [8]string{}
	for i := 0; i < 8; i++ {
		proofHexNumbers[i] = toHex(new(big.Int).SetBytes(proofBytes[i*fpSize : (i+1)*fpSize]))
	}

	proofJson.Ar = [2]string{proofHexNumbers[0], proofHexNumbers[1]}
	proofJson.Bs = [2][2]string{
		{proofHexNumbers[2], proofHexNumbers[3]},
		{proofHexNumbers[4], proofHexNumbers[5]},
	}
	proofJson.Krs = [2]string{proofHexNumbers[6], proofHexNumbers[7]}

	return json.Marshal(proofJson)
}

func (p *Proof) UnmarshalJSON(data []byte) error {
	var proofJson ProofJSON
	err := json.Unmarshal(data, &proofJson)
	if err != nil {
		return err
	}
	proofHexNumbers := [8]string{
		proofJson.Ar[0],
		proofJson.Ar[1],
		proofJson.Bs[0][0],
		proofJson.Bs[0][1],
		proofJson.Bs[1][0],
		proofJson.Bs[1][1],
		proofJson.Krs[0],
		proofJson.Krs[1],
	}
	proofInts := [8]big.Int{}
	for i := 0; i < 8; i++ {
		err = fromHex(&proofInts[i], proofHexNumbers[i])
		if err != nil {
			return err
		}
	}
	const fpSize = 32
	proofBytes := make([]byte, 8*fpSize)
	for i := 0; i < 8; i++ {
		intBytes := proofInts[i].Bytes()
		// Pad with leading zeros to ensure exactly 32 bytes
		if len(intBytes) <= fpSize {
			copy(proofBytes[i*fpSize+fpSize-len(intBytes):(i+1)*fpSize], intBytes)
		} else {
			// If somehow longer than 32 bytes, take the last 32 bytes
			copy(proofBytes[i*fpSize:(i+1)*fpSize], intBytes[len(intBytes)-fpSize:])
		}
	}

	p.Proof = groth16.NewProof(ecc.BN254)

	_, err = p.Proof.ReadFrom(bytes.NewReader(proofBytes))
	if err != nil {
		return err
	}
	return nil
}

func (ps *ProvingSystemV1) WriteTo(w io.Writer) (int64, error) {
	var totalWritten int64 = 0
	var intBuf [4]byte

	fieldsToWrite := []uint32{
		ps.InclusionTreeHeight,
		ps.InclusionNumberOfCompressedAccounts,
		ps.NonInclusionTreeHeight,
		ps.NonInclusionNumberOfCompressedAccounts,
	}

	for _, field := range fieldsToWrite {
		binary.BigEndian.PutUint32(intBuf[:], field)
		written, err := w.Write(intBuf[:])
		totalWritten += int64(written)
		if err != nil {
			return totalWritten, err
		}
	}

	keyWritten, err := ps.ProvingKey.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}

	keyWritten, err = ps.VerifyingKey.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}

	keyWritten, err = ps.ConstraintSystem.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}
	return totalWritten, nil
}

func (ps *ProvingSystemV1) UnsafeReadFrom(r io.Reader) (int64, error) {
	var totalRead int64 = 0
	var intBuf [4]byte

	fieldsToRead := []*uint32{
		&ps.InclusionTreeHeight,
		&ps.InclusionNumberOfCompressedAccounts,
		&ps.NonInclusionTreeHeight,
		&ps.NonInclusionNumberOfCompressedAccounts,
	}

	for _, field := range fieldsToRead {
		read, err := io.ReadFull(r, intBuf[:])
		totalRead += int64(read)
		if err != nil {
			return totalRead, err
		}
		*field = binary.BigEndian.Uint32(intBuf[:])
	}

	ps.ProvingKey = groth16.NewProvingKey(ecc.BN254)
	keyRead, err := ps.ProvingKey.UnsafeReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	ps.VerifyingKey = groth16.NewVerifyingKey(ecc.BN254)
	keyRead, err = ps.VerifyingKey.UnsafeReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	ps.ConstraintSystem = groth16.NewCS(ecc.BN254)
	keyRead, err = ps.ConstraintSystem.ReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	return totalRead, nil
}

func ReadSystemFromFile(path string) (interface{}, error) {
	if strings.Contains(strings.ToLower(path), "address-append") {
		ps := new(ProvingSystemV2)
		ps.CircuitType = BatchAddressAppendCircuitType
		file, err := os.Open(path)
		if err != nil {
			return nil, err
		}
		defer file.Close()

		_, err = ps.UnsafeReadFrom(file)
		if err != nil {
			return nil, err
		}
		return ps, nil
	} else if strings.Contains(strings.ToLower(path), "append") {
		ps := new(ProvingSystemV2)
		ps.CircuitType = BatchAppendCircuitType
		file, err := os.Open(path)
		if err != nil {
			return nil, err
		}
		defer file.Close()
		_, err = ps.UnsafeReadFrom(file)
		if err != nil {
			return nil, err
		}
		return ps, nil
	} else if strings.Contains(strings.ToLower(path), "update") {
		ps := new(ProvingSystemV2)
		ps.CircuitType = BatchUpdateCircuitType
		file, err := os.Open(path)
		if err != nil {
			return nil, err
		}
		defer file.Close()

		_, err = ps.UnsafeReadFrom(file)
		if err != nil {
			return nil, err
		}
		return ps, nil
	} else {
		ps := new(ProvingSystemV1)
		if strings.Contains(strings.ToLower(path), "mainnet") {
			ps.Version = 0
		} else {
			ps.Version = 1
		}
		file, err := os.Open(path)
		if err != nil {
			return nil, err
		}
		defer file.Close()

		_, err = ps.UnsafeReadFrom(file)
		if err != nil {
			return nil, err
		}
		return ps, nil
	}
}

func (ps *ProvingSystemV2) WriteTo(w io.Writer) (int64, error) {
	var totalWritten int64 = 0
	var intBuf [4]byte

	fieldsToWrite := []uint32{
		ps.TreeHeight,
		ps.BatchSize,
	}

	for _, field := range fieldsToWrite {
		binary.BigEndian.PutUint32(intBuf[:], field)
		written, err := w.Write(intBuf[:])
		totalWritten += int64(written)
		if err != nil {
			return totalWritten, err
		}
	}

	keyWritten, err := ps.ProvingKey.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}

	keyWritten, err = ps.VerifyingKey.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}

	keyWritten, err = ps.ConstraintSystem.WriteTo(w)
	totalWritten += keyWritten
	if err != nil {
		return totalWritten, err
	}

	return totalWritten, nil
}

func (ps *ProvingSystemV2) UnsafeReadFrom(r io.Reader) (int64, error) {
	var totalRead int64 = 0
	var intBuf [4]byte

	fieldsToRead := []*uint32{
		&ps.TreeHeight,
		&ps.BatchSize,
	}

	for _, field := range fieldsToRead {
		read, err := io.ReadFull(r, intBuf[:])
		totalRead += int64(read)
		if err != nil {
			return totalRead, err
		}
		*field = binary.BigEndian.Uint32(intBuf[:])
	}

	ps.ProvingKey = groth16.NewProvingKey(ecc.BN254)
	keyRead, err := ps.ProvingKey.UnsafeReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	ps.VerifyingKey = groth16.NewVerifyingKey(ecc.BN254)
	keyRead, err = ps.VerifyingKey.UnsafeReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	ps.ConstraintSystem = groth16.NewCS(ecc.BN254)
	keyRead, err = ps.ConstraintSystem.ReadFrom(r)
	totalRead += keyRead
	if err != nil {
		return totalRead, err
	}

	return totalRead, nil
}
