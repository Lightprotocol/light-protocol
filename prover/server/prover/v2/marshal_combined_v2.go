package v2

import (
	"encoding/json"
	"fmt"
	"light/light-prover/prover/common"
)

type V2CombinedParametersJSON struct {
	CircuitType             common.CircuitType            `json:"circuitType"`
	StateTreeHeight         uint32                        `json:"stateTreeHeight"`
	AddressTreeHeight       uint32                        `json:"addressTreeHeight"`
	PublicInputHash         string                        `json:"publicInputHash"`
	InclusionProofInputs    []InclusionProofInputsJSON    `json:"inputCompressedAccounts"`
	NonInclusionProofInputs []NonInclusionProofInputsJSON `json:"newAddresses"`
}

func ParseCombined(inputJSON string) (V2NonInclusionParameters, error) {
	var proofData V2NonInclusionParameters
	err := json.Unmarshal([]byte(inputJSON), &proofData)
	if err != nil {
		return V2NonInclusionParameters{}, fmt.Errorf("error parsing JSON: %v", err)
	}
	return proofData, nil
}

func (p *V2CombinedParameters) MarshalJSON() ([]byte, error) {
	combined := V2CombinedParametersJSON{
		CircuitType:             common.CombinedCircuitType,
		PublicInputHash:         common.ToHex(&p.PublicInputHash),
		StateTreeHeight:         uint32(len(p.InclusionParameters.Inputs[0].PathElements)),
		AddressTreeHeight:       uint32(len(p.NonInclusionParameters.Inputs[0].PathElements)),
		InclusionProofInputs:    p.InclusionParameters.CreateV2InclusionParametersJSON().InclusionInputs,
		NonInclusionProofInputs: p.NonInclusionParameters.CreateV2NonInclusionParametersJSON().NonInclusionInputs,
	}
	return json.Marshal(combined)
}

func (p *V2CombinedParameters) UnmarshalJSON(data []byte) error {
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
	common.FromHex(&p.PublicInputHash, publicInputHash)

	if _, ok := rawMessages["inputCompressedAccounts"]; ok {
		var params V2InclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.InclusionParameters = V2InclusionParameters{Inputs: nil}
		err = p.InclusionParameters.UpdateWithJSON(params)
		if err != nil {
			return err
		}
	}

	if _, ok := rawMessages["newAddresses"]; ok {
		var params V2NonInclusionParametersJSON
		err := json.Unmarshal(data, &params)
		if err != nil {
			return err
		}
		p.NonInclusionParameters = V2NonInclusionParameters{Inputs: nil}
		err = p.NonInclusionParameters.UpdateWithJSON(params, err)
		if err != nil {
			return err
		}
	}

	return nil
}
