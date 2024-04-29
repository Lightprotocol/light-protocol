package prover

import (
	"encoding/json"
	"fmt"
)

type CombinedParametersJSON struct {
	InclusionParametersJSON    `json:"inclusion"`
	NonInclusionParametersJSON `json:"nonInclusion"`
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
	inclusionParams, err := p.InclusionParameters.MarshalJSON()
	if err != nil {
		return nil, err
	}

	nonInclusionParams, err := p.NonInclusionParameters.MarshalJSON()
	if err != nil {
		return nil, err
	}

	combined := map[string]json.RawMessage{
		"inclusion":    json.RawMessage(inclusionParams),
		"nonInclusion": json.RawMessage(nonInclusionParams),
	}

	return json.Marshal(combined)
}

func (p *CombinedParameters) UnmarshalJSON(data []byte) error {
	var rawMessages map[string]json.RawMessage
	err := json.Unmarshal(data, &rawMessages)
	if err != nil {
		return err
	}

	if msg, ok := rawMessages["inclusion"]; ok {
		err := p.InclusionParameters.UnmarshalJSON(msg)
		if err != nil {
			return err
		}
	}

	if msg, ok := rawMessages["nonInclusion"]; ok {
		err := p.NonInclusionParameters.UnmarshalJSON(msg)
		if err != nil {
			return err
		}
	}

	return nil
}
