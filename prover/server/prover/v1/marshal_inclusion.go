package v1

import (
	"encoding/json"
	"light/light-prover/prover/common"
	"math/big"
)

type InclusionParametersJSON struct {
	CircuitType     string                            `json:"circuitType"`
	StateTreeHeight uint32                            `json:"stateTreeHeight"`
	Inputs          []common.InclusionProofInputsJSON `json:"inputCompressedAccounts"`
}

func (p *InclusionParameters) MarshalJSON() ([]byte, error) {
	paramsJson := p.CreateInclusionParametersJSON()
	return json.Marshal(paramsJson)
}

func (p *InclusionParameters) CreateInclusionParametersJSON() InclusionParametersJSON {
	paramsJson := InclusionParametersJSON{}
	paramsJson.Inputs = make([]common.InclusionProofInputsJSON, p.NumberOfCompressedAccounts())
	paramsJson.CircuitType = string(common.InclusionCircuitType)
	paramsJson.StateTreeHeight = 26 // v1 always uses height 26
	for i := 0; i < int(p.NumberOfCompressedAccounts()); i++ {
		paramsJson.Inputs[i].Root = common.ToHex(&p.Inputs[i].Root)
		paramsJson.Inputs[i].Leaf = common.ToHex(&p.Inputs[i].Leaf)
		paramsJson.Inputs[i].PathIndex = p.Inputs[i].PathIndex
		paramsJson.Inputs[i].PathElements = make([]string, len(p.Inputs[i].PathElements))
		for j := 0; j < len(p.Inputs[i].PathElements); j++ {
			paramsJson.Inputs[i].PathElements[j] = common.ToHex(&p.Inputs[i].PathElements[j])
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
	p.Inputs = make([]InclusionInputs, len(params.Inputs))
	for i := 0; i < len(params.Inputs); i++ {
		err := common.FromHex(&p.Inputs[i].Root, params.Inputs[i].Root)
		if err != nil {
			return err
		}
		err = common.FromHex(&p.Inputs[i].Leaf, params.Inputs[i].Leaf)
		if err != nil {
			return err
		}
		p.Inputs[i].PathIndex = params.Inputs[i].PathIndex
		p.Inputs[i].PathElements = make([]big.Int, len(params.Inputs[i].PathElements))
		for j := 0; j < len(params.Inputs[i].PathElements); j++ {
			err = common.FromHex(&p.Inputs[i].PathElements[j], params.Inputs[i].PathElements[j])
			if err != nil {
				return err
			}
		}
	}
	return nil
}
