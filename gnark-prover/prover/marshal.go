package prover

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"math/big"
	"os"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
)

func fromHex(i *big.Int, s string) error {
	_, ok := i.SetString(s, 0)
	if !ok {
		return fmt.Errorf("invalid number: %s", s)
	}
	return nil
}

func toHex(i *big.Int) string {
	return fmt.Sprintf("0x%s", i.Text(16))
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
		copy(proofBytes[i*fpSize:(i+1)*fpSize], proofInts[i].Bytes())
	}

	p.Proof = groth16.NewProof(ecc.BN254)

	_, err = p.Proof.ReadFrom(bytes.NewReader(proofBytes))
	if err != nil {
		return err
	}
	return nil
}

func (ps *ProvingSystem) WriteTo(w io.Writer) (int64, error) {
	var totalWritten int64 = 0
	var intBuf [4]byte

	binary.BigEndian.PutUint32(intBuf[:], ps.TreeDepth)
	written, err := w.Write(intBuf[:])
	totalWritten += int64(written)
	if err != nil {
		return totalWritten, err
	}

	binary.BigEndian.PutUint32(intBuf[:], ps.NumberOfUtxos)
	written, err = w.Write(intBuf[:])
	totalWritten += int64(written)
	if err != nil {
		return totalWritten, err
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

func (ps *ProvingSystem) UnsafeReadFrom(r io.Reader) (int64, error) {
	var totalRead int64 = 0
	var intBuf [4]byte

	read, err := io.ReadFull(r, intBuf[:])
	totalRead += int64(read)
	if err != nil {
		return totalRead, err
	}
	ps.TreeDepth = binary.BigEndian.Uint32(intBuf[:])

	read, err = io.ReadFull(r, intBuf[:])
	totalRead += int64(read)
	if err != nil {
		return totalRead, err
	}
	ps.NumberOfUtxos = binary.BigEndian.Uint32(intBuf[:])

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

func ReadSystemFromFile(path string) (ps *ProvingSystem, err error) {
	ps = new(ProvingSystem)
	file, err := os.Open(path)
	if err != nil {
		return
	}

	defer func() {
		closeErr := file.Close()
		if closeErr != nil && err == nil {
			err = closeErr
		}
	}()

	_, err = ps.UnsafeReadFrom(file)
	if err != nil {
		return
	}
	return
}

type InclusionParametersJSON struct {
	Root           []string   `json:"root"`
	InPathIndices  []uint32   `json:"inPathIndices"`
	InPathElements [][]string `json:"inPathElements"`
	Leaf           []string   `json:"leaf"`
}

func ParseInput(inputJSON string) (InclusionParameters, error) {
	var proofData InclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return InclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *InclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := InclusionParametersJSON{}

	paramsJson.Root = make([]string, len(p.Root))
	for i := 0; i < len(p.Root); i++ {
		paramsJson.Root[i] = toHex(&p.Root[i])
	}

	paramsJson.InPathIndices = make([]uint32, len(p.InPathIndices))
	for i := 0; i < len(p.InPathIndices); i++ {
		paramsJson.InPathIndices[i] = p.InPathIndices[i]
	}

	paramsJson.InPathElements = make([][]string, len(p.InPathElements))
	for i := 0; i < len(p.InPathElements); i++ {
		paramsJson.InPathElements[i] = make([]string, len(p.InPathElements[i]))
		for j := 0; j < len(p.InPathElements[i]); j++ {
			paramsJson.InPathElements[i][j] = toHex(&p.InPathElements[i][j])
		}
	}

	paramsJson.Leaf = make([]string, len(p.Leaf))
	for i := 0; i < len(p.Leaf); i++ {
		paramsJson.Leaf[i] = toHex(&p.Leaf[i])
	}

	return json.Marshal(paramsJson)
}

func (p *InclusionParameters) UnmarshalJSON(data []byte) error {

	var params InclusionParametersJSON

	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	p.Root = make([]big.Int, len(params.Root))
	for i := 0; i < len(params.Root); i++ {
		err = fromHex(&p.Root[i], params.Root[i])
		if err != nil {
			return err
		}
	}

	p.Leaf = make([]big.Int, len(params.Leaf))
	for i := 0; i < len(params.Leaf); i++ {
		err = fromHex(&p.Leaf[i], params.Leaf[i])
		if err != nil {
			return err
		}
	}

	p.InPathIndices = make([]uint32, len(params.InPathIndices))
	for i := 0; i < len(params.InPathIndices); i++ {
		p.InPathIndices[i] = params.InPathIndices[i]
	}

	p.InPathElements = make([][]big.Int, len(params.InPathElements))
	for i := 0; i < len(params.InPathElements); i++ {
		p.InPathElements[i] = make([]big.Int, len(params.InPathElements[i]))
		for j := 0; j < len(params.InPathElements[i]); j++ {
			err = fromHex(&p.InPathElements[i][j], params.InPathElements[i][j])
			if err != nil {
				return err
			}
		}
	}

	return nil
}
