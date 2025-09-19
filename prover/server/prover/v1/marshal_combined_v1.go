package v1

import (
	"encoding/json"
	"fmt"
	"light/light-prover/prover/common"
)

type V1CombinedParametersJSON struct {
	CircuitType             common.CircuitType                   `json:"circuitType"`
	StateTreeHeight         uint32                               `json:"stateTreeHeight"`
	AddressTreeHeight       uint32                               `json:"addressTreeHeight"`
	InclusionProofInputs    []common.InclusionProofInputsJSON    `json:"inputCompressedAccounts"`
	NonInclusionProofInputs []common.NonInclusionProofInputsJSON `json:"newAddresses"`
}

func V1ParseCombined(inputJSON string) (V1CombinedParameters, error) {
	var proofData V1CombinedParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return V1CombinedParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *V1CombinedParameters) UnmarshalJSON(data []byte) error {
	var rawMessages map[string]json.RawMessage
	err := json.Unmarshal(data, &rawMessages)
	if err != nil {
		return err
	}

	if _, ok := rawMessages["inputCompressedAccounts"]; ok {
		var params V1InclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.InclusionParameters = V1InclusionParameters{Inputs: nil}
		err = p.InclusionParameters.UpdateWithJSON(params)
		if err != nil {
			return err
		}
	}

	if _, ok := rawMessages["newAddresses"]; ok {
		var params V1NonInclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.NonInclusionParameters = V1NonInclusionParameters{Inputs: nil}
		err = p.NonInclusionParameters.UpdateWithJSON(params, err)
		if err != nil {
			return err
		}
	}

	return nil
}
