package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type BatchAppendWithSubtreesParametersJSON struct {
	PublicInputHash     string   `json:"publicInputHash"`
	OldSubTreeHashChain string   `json:"oldSubTreeHashChain"`
	NewSubTreeHashChain string   `json:"newSubTreeHashChain"`
	NewRoot             string   `json:"newRoot"`
	HashchainHash       string   `json:"hashchainHash"`
	StartIndex          uint32   `json:"startIndex"`
	Leaves              []string `json:"leaves"`
	Subtrees            []string `json:"subtrees"`
	TreeHeight          uint32   `json:"treeHeight"`
}

func ParseBatchAppendInput(inputJSON string) (BatchAppendWithSubtreesParameters, error) {
	var proofData BatchAppendWithSubtreesParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return BatchAppendWithSubtreesParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *BatchAppendWithSubtreesParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateBatchAppendWithSubtreesParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *BatchAppendWithSubtreesParameters) CreateBatchAppendWithSubtreesParametersJSON() BatchAppendWithSubtreesParametersJSON {
	paramsJson := BatchAppendWithSubtreesParametersJSON{
		PublicInputHash:     toHex(p.PublicInputHash),
		OldSubTreeHashChain: toHex(p.OldSubTreeHashChain),
		NewSubTreeHashChain: toHex(p.NewSubTreeHashChain),
		NewRoot:             toHex(p.NewRoot),
		HashchainHash:       toHex(p.HashchainHash),
		StartIndex:          p.StartIndex,
		TreeHeight:          p.TreeHeight,
	}

	paramsJson.Leaves = make([]string, len(p.Leaves))
	for i, leaf := range p.Leaves {
		paramsJson.Leaves[i] = toHex(leaf)
	}

	paramsJson.Subtrees = make([]string, len(p.Subtrees))
	for i, subtree := range p.Subtrees {
		paramsJson.Subtrees[i] = toHex(subtree)
	}

	return paramsJson
}

func (p *BatchAppendWithSubtreesParameters) UnmarshalJSON(data []byte) error {
	var params BatchAppendWithSubtreesParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}
	return p.UpdateWithJSON(params)
}

func (p *BatchAppendWithSubtreesParameters) UpdateWithJSON(params BatchAppendWithSubtreesParametersJSON) error {
	var err error

	p.TreeHeight = params.TreeHeight

	p.PublicInputHash = new(big.Int)
	err = fromHex(p.PublicInputHash, params.PublicInputHash)
	if err != nil {
		return err
	}

	p.OldSubTreeHashChain = new(big.Int)
	err = fromHex(p.OldSubTreeHashChain, params.OldSubTreeHashChain)
	if err != nil {
		return err
	}

	p.NewSubTreeHashChain = new(big.Int)
	err = fromHex(p.NewSubTreeHashChain, params.NewSubTreeHashChain)
	if err != nil {
		return err
	}

	p.NewRoot = new(big.Int)
	err = fromHex(p.NewRoot, params.NewRoot)
	if err != nil {
		return err
	}

	p.HashchainHash = new(big.Int)
	err = fromHex(p.HashchainHash, params.HashchainHash)
	if err != nil {
		return err
	}

	p.StartIndex = params.StartIndex

	p.Leaves = make([]*big.Int, len(params.Leaves))
	for i, leafStr := range params.Leaves {
		p.Leaves[i] = new(big.Int)
		err = fromHex(p.Leaves[i], leafStr)
		if err != nil {
			return err
		}
	}

	p.Subtrees = make([]*big.Int, len(params.Subtrees))
	for i, subtreeStr := range params.Subtrees {
		p.Subtrees[i] = new(big.Int)
		err = fromHex(p.Subtrees[i], subtreeStr)
		if err != nil {
			return err
		}
	}

	return nil
}
