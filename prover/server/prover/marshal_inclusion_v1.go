package prover

import (
	"encoding/json"
	"math/big"
)

type V1InclusionParametersJSON struct {
	CircuitType     string                     `json:"circuitType"`
	StateTreeHeight uint32                     `json:"stateTreeHeight"`
	Inputs          []InclusionProofInputsJSON `json:"inputCompressedAccounts"`
}

func (p *V1InclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *V1InclusionParameters) CreateInclusionParametersJSON() V1InclusionParametersJSON {
	paramsJson := V1InclusionParametersJSON{}
	paramsJson.Inputs = make([]InclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	paramsJson.CircuitType = string(CombinedCircuitType)
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.Inputs[i].Root = toHex(&p.Inputs[i].Root)
		paramsJson.Inputs[i].Leaf = toHex(&p.Inputs[i].Leaf)
		paramsJson.Inputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.Inputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.Inputs[i].PathElements[j] = toHex(&p.Inputs[i].PathElements[j])
		}
	}
	return paramsJson
}

func (p *V1InclusionParameters) UnmarshalJSON(data []byte) error {
	var params V1InclusionParametersJSON
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

func (p *V1InclusionParameters) UpdateWithJSON(params V1InclusionParametersJSON) error {
	p.Inputs = make([]InclusionInputs, len(params.Inputs))
	for i := 0; i < len(params.Inputs); i++ {
		err := fromHex(&p.Inputs[i].Root, params.Inputs[i].Root)
		if err != nil {
			return err
		}
		err = fromHex(&p.Inputs[i].Leaf, params.Inputs[i].Leaf)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.Inputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.Inputs[i].PathElements))
		for j := 0; j < len(params.Inputs[i].PathElements); j++ {
			err = fromHex(&p.Inputs[i].PathElements[j], params.Inputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
	}
	return nil
}
