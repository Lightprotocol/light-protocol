package prover

import (
	"encoding/json"
	"fmt"
)

type CombinedParametersJSON struct {
	CircuitType             CircuitType                   `json:"circuitType"`
	PublicInputHash         string                        `json:"publicInputHash"`
	InclusionProofInputs    []InclusionProofInputsJSON    `json:"input-compressed-accounts"`
	NonInclusionProofInputs []NonInclusionProofInputsJSON `json:"new-addresses"`
}

func ParseCombined(inputJSON string) (NonInclusionParameters, error) {
	var proofData NonInclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return NonInclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *CombinedParameters) MarshalJSON() ([]byte, error) {
	combined := CombinedParametersJSON{
		CircuitType:             CombinedCircuitType,
		PublicInputHash:         toHex(&p.PublicInputHash),
		InclusionProofInputs:    p.InclusionParameters.CreateInclusionParametersJSON().Inputs,
		NonInclusionProofInputs: p.NonInclusionParameters.CreateNonInclusionParametersJSON().Inputs,
	}
	return json.Marshal(combined)
}

func (p *CombinedParameters) UnmarshalJSON(data []byte) error {
	var rawMessages map[string]json.RawMessage
	err := json.Unmarshal(data, &rawMessages)
	if err != nil {
		return err
	}
	var publicInputHash string
	err1 := json.Unmarshal(rawMessages["publicInputHash"], &publicInputHash)
	if err1 != nil {
		return fmt.Errorf("failed to unmarshal publicInputHash: %v", err)
	}
	fromHex(&p.PublicInputHash, publicInputHash)

	if _, ok := rawMessages["input-compressed-accounts"]; ok {
		var params InclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.InclusionParameters = InclusionParameters{Inputs: nil}
		err = p.InclusionParameters.UpdateWithJSON(params)
		if err != nil {
			return err
		}
	}

	if _, ok := rawMessages["new-addresses"]; ok {
		var params NonInclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.NonInclusionParameters = NonInclusionParameters{Inputs: nil}
		err = p.NonInclusionParameters.UpdateWithJSON(params, err)
		if err != nil {
			return err
		}
	}

	return nil
}
