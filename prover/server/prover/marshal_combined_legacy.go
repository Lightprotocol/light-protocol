package prover

import (
	"encoding/json"
	"fmt"
)

type LegacyCombinedParametersJSON struct {
	CircuitType             CircuitType                   `json:"circuitType"`
	StateTreeHeight         uint32                        `json:"stateTreeHeight"`
	AddressTreeHeight       uint32                        `json:"addressTreeHeight"`
	InclusionProofInputs    []InclusionProofInputsJSON    `json:"inputCompressedAccounts"`
	NonInclusionProofInputs []NonInclusionProofInputsJSON `json:"newAddresses"`
}

func LegacyParseCombined(inputJSON string) (LegacyCombinedParameters, error) {
	var proofData LegacyCombinedParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return LegacyCombinedParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *LegacyCombinedParameters) UnmarshalJSON(data []byte) error {
	var rawMessages map[string]json.RawMessage
	err := json.Unmarshal(data, &rawMessages)
	if err != nil {
		return err
	}

	if _, ok := rawMessages["inputCompressedAccounts"]; ok {
		var params LegacyInclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.InclusionParameters = LegacyInclusionParameters{Inputs: nil}
		err = p.InclusionParameters.UpdateWithJSON(params)
		if err != nil {
			return err
		}
	}

	if _, ok := rawMessages["newAddresses"]; ok {
		var params LegacyNonInclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.NonInclusionParameters = LegacyNonInclusionParameters{Inputs: nil}
		err = p.NonInclusionParameters.UpdateWithJSON(params, err)
		if err != nil {
			return err
		}
	}

	return nil
}
