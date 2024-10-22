package prover

import (
	"encoding/json"
	"fmt"
	merkletree "light/light-prover/merkle-tree"
	"math/big"
)

type BatchAddressAppendProofInputsJSON struct {
	PublicInputHash  string               `json:"publicInputHash"`
	OldRoot          string               `json:"oldRoot"`
	NewRoot          string               `json:"newRoot"`
	HashchainHash    string               `json:"hashchainHash"`
	StartIndex       uint32               `json:"startIndex"`
	OldLowElements   []IndexedElementJSON `json:"oldLowElements"`
	LowElements      []IndexedElementJSON `json:"lowElements"`
	NewElements      []IndexedElementJSON `json:"newElements"`
	LowElementProofs [][]string           `json:"lowElementProofs"`
	NewElementProofs [][]string           `json:"newElementProofs"`
	TreeHeight       uint32               `json:"treeHeight"`
	BatchSize        uint32               `json:"batchSize"`
}

type IndexedElementJSON struct {
	Value     string `json:"value"`
	NextValue string `json:"nextValue"`
	NextIndex uint32 `json:"nextIndex"`
	Index     uint32 `json:"index"`
}

func ParseBatchAddressAppendInput(inputJSON string) (BatchAddressTreeAppendParameters, error) {
	var proofData BatchAddressTreeAppendParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return BatchAddressTreeAppendParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *BatchAddressTreeAppendParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateBatchAddressAppendParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *BatchAddressTreeAppendParameters) CreateBatchAddressAppendParametersJSON() BatchAddressAppendProofInputsJSON {
	paramsJson := BatchAddressAppendProofInputsJSON{}
	paramsJson.PublicInputHash = toHex(p.PublicInputHash)
	paramsJson.OldRoot = toHex(p.OldRoot)
	paramsJson.NewRoot = toHex(p.NewRoot)
	paramsJson.HashchainHash = toHex(p.HashchainHash)
	paramsJson.StartIndex = p.StartIndex
	paramsJson.TreeHeight = p.TreeHeight
	paramsJson.BatchSize = p.BatchSize

	// Convert OldLowElements
	paramsJson.OldLowElements = make([]IndexedElementJSON, len(p.OldLowElements))
	for i := 0; i < len(p.OldLowElements); i++ {
		paramsJson.OldLowElements[i] = IndexedElementJSON{
			Value:     toHex(p.OldLowElements[i].Value),
			NextValue: toHex(p.OldLowElements[i].NextValue),
			NextIndex: p.OldLowElements[i].NextIndex,
			Index:     p.OldLowElements[i].Index,
		}
	}

	// Convert LowElements
	paramsJson.LowElements = make([]IndexedElementJSON, len(p.LowElements))
	for i := 0; i < len(p.LowElements); i++ {
		paramsJson.LowElements[i] = IndexedElementJSON{
			Value:     toHex(p.LowElements[i].Value),
			NextValue: toHex(p.LowElements[i].NextValue),
			NextIndex: p.LowElements[i].NextIndex,
			Index:     p.LowElements[i].Index,
		}
	}

	// Convert NewElements
	paramsJson.NewElements = make([]IndexedElementJSON, len(p.NewElements))
	for i := 0; i < len(p.NewElements); i++ {
		paramsJson.NewElements[i] = IndexedElementJSON{
			Value:     toHex(p.NewElements[i].Value),
			NextValue: toHex(p.NewElements[i].NextValue),
			NextIndex: p.NewElements[i].NextIndex,
			Index:     p.NewElements[i].Index,
		}
	}

	// Convert LowElementProofs
	paramsJson.LowElementProofs = make([][]string, len(p.LowElementProofs))
	for i := 0; i < len(p.LowElementProofs); i++ {
		paramsJson.LowElementProofs[i] = make([]string, len(p.LowElementProofs[i]))
		for j := 0; j < len(p.LowElementProofs[i]); j++ {
			paramsJson.LowElementProofs[i][j] = toHex(&p.LowElementProofs[i][j])
		}
	}

	// Convert NewElementProofs
	paramsJson.NewElementProofs = make([][]string, len(p.NewElementProofs))
	for i := 0; i < len(p.NewElementProofs); i++ {
		paramsJson.NewElementProofs[i] = make([]string, len(p.NewElementProofs[i]))
		for j := 0; j < len(p.NewElementProofs[i]); j++ {
			paramsJson.NewElementProofs[i][j] = toHex(&p.NewElementProofs[i][j])
		}
	}

	return paramsJson
}

func (p *BatchAddressTreeAppendParameters) UnmarshalJSON(data []byte) error {
	var params BatchAddressAppendProofInputsJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	return p.UpdateWithJSON(params)
}

func (p *BatchAddressTreeAppendParameters) UpdateWithJSON(params BatchAddressAppendProofInputsJSON) error {
	var err error
	p.TreeHeight = params.TreeHeight
	p.BatchSize = params.BatchSize
	p.StartIndex = params.StartIndex

	// Parse OldRoot
	p.OldRoot = new(big.Int)
	err = fromHex(p.OldRoot, params.OldRoot)
	if err != nil {
		return err
	}

	// Parse NewRoot
	p.NewRoot = new(big.Int)
	err = fromHex(p.NewRoot, params.NewRoot)
	if err != nil {
		return err
	}

	// Parse HashchainHash
	p.HashchainHash = new(big.Int)
	err = fromHex(p.HashchainHash, params.HashchainHash)
	if err != nil {
		return err
	}

	// Parse OldLowElements
	p.OldLowElements = make([]merkletree.IndexedElement, len(params.OldLowElements))
	for i := 0; i < len(params.OldLowElements); i++ {
		err = fromHex(p.OldLowElements[i].Value, params.OldLowElements[i].Value)
		if err != nil {
			return err
		}
		p.OldLowElements[i].NextValue = new(big.Int)
		err = fromHex(p.OldLowElements[i].NextValue, params.OldLowElements[i].NextValue)
		if err != nil {
			return err
		}
		p.OldLowElements[i].NextIndex = params.OldLowElements[i].NextIndex
		p.OldLowElements[i].Index = params.OldLowElements[i].Index
	}

	// Parse LowElements
	p.LowElements = make([]merkletree.IndexedElement, len(params.LowElements))
	for i := 0; i < len(params.LowElements); i++ {
		err = fromHex(p.LowElements[i].Value, params.LowElements[i].Value)
		if err != nil {
			return err
		}
		p.LowElements[i].NextValue = new(big.Int)
		err = fromHex(p.LowElements[i].NextValue, params.LowElements[i].NextValue)
		if err != nil {
			return err
		}
		p.LowElements[i].NextIndex = params.LowElements[i].NextIndex
		p.LowElements[i].Index = params.LowElements[i].Index
	}

	// Parse NewElements
	p.NewElements = make([]merkletree.IndexedElement, len(params.NewElements))
	for i := 0; i < len(params.NewElements); i++ {
		err = fromHex(p.NewElements[i].Value, params.NewElements[i].Value)
		if err != nil {
			return err
		}
		p.NewElements[i].NextValue = new(big.Int)
		err = fromHex(p.NewElements[i].NextValue, params.NewElements[i].NextValue)
		if err != nil {
			return err
		}
		p.NewElements[i].NextIndex = params.NewElements[i].NextIndex
		p.NewElements[i].Index = params.NewElements[i].Index
	}

	// Parse LowElementProofs
	p.LowElementProofs = make([][]big.Int, len(params.LowElementProofs))
	for i := 0; i < len(params.LowElementProofs); i++ {
		p.LowElementProofs[i] = make([]big.Int, len(params.LowElementProofs[i]))
		for j := 0; j < len(params.LowElementProofs[i]); j++ {
			err = fromHex(&p.LowElementProofs[i][j], params.LowElementProofs[i][j])
			if err != nil {
				return err
			}
		}
	}

	// Parse NewElementProofs
	p.NewElementProofs = make([][]big.Int, len(params.NewElementProofs))
	for i := 0; i < len(params.NewElementProofs); i++ {
		p.NewElementProofs[i] = make([]big.Int, len(params.NewElementProofs[i]))
		for j := 0; j < len(params.NewElementProofs[i]); j++ {
			err = fromHex(&p.NewElementProofs[i][j], params.NewElementProofs[i][j])
			if err != nil {
				return err
			}
		}
	}

	// Parse PublicInputHash
	p.PublicInputHash = new(big.Int)
	err = fromHex(p.PublicInputHash, params.PublicInputHash)
	if err != nil {
		return err
	}

	return nil
}
