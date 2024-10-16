package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
)

type InclusionProofInputsJSON struct {
	Root         string   `json:"root"`
	PathIndex    uint32   `json:"pathIndex"`
	PathElements []string `json:"pathElements"`
	Leaf         string   `json:"leaf"`
}

type InclusionParametersJSON struct {
	CircuitType     string                     `json:"circuitType"`
	StateTreeHeight uint32                     `json:"stateTreeHeight"`
	PublicInputHash string                     `json:"publicInputHash"`
	InclusionInputs []InclusionProofInputsJSON `json:"inputCompressedAccounts"`
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
	paramsJson := p.CreateInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *InclusionParameters) CreateInclusionParametersJSON() InclusionParametersJSON {
	paramsJson := InclusionParametersJSON{}
	paramsJson.InclusionInputs = make([]InclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	paramsJson.PublicInputHash = toHex(&p.PublicInputHash)
	paramsJson.CircuitType = string(CombinedCircuitType)
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.InclusionInputs[i].Root = toHex(&p.Inputs[i].Root)
		paramsJson.InclusionInputs[i].Leaf = toHex(&p.Inputs[i].Leaf)
		paramsJson.InclusionInputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.InclusionInputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.InclusionInputs[i].PathElements[j] = toHex(&p.Inputs[i].PathElements[j])
		}
	}
	return paramsJson
}

func (p *InclusionParameters) UnmarshalJSON(data []byte) error {
	var params InclusionParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}
	err = p.UpdateWithJSON(params)
	if err != nil {
		return err
	}
	return nil
}

func (p *InclusionParameters) UpdateWithJSON(params InclusionParametersJSON) error {
	fromHex(&p.PublicInputHash, params.PublicInputHash)
	p.Inputs = make([]InclusionInputs, len(params.InclusionInputs))
	for i := 0; i < len(params.InclusionInputs); i++ {
		err := fromHex(&p.Inputs[i].Root, params.InclusionInputs[i].Root)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].Leaf, params.InclusionInputs[i].Leaf)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.InclusionInputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.InclusionInputs[i].PathElements))
		for j := 0; j < len(params.InclusionInputs[i].PathElements); j++ {
			err = fromHex(&p.Inputs[i].PathElements[j], params.InclusionInputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
	}
	return nil
}
