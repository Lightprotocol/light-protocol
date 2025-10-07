package v1

import (
	"encoding/json"
	"fmt"
	"light/light-prover/prover/common"
)

// CombinedParametersJSON represents the v1 combined format
type CombinedParametersJSON struct {
	CircuitType             string                            `json:"circuitType"`
	StateTreeHeight         uint32                            `json:"stateTreeHeight"`
	AddressTreeHeight       uint32                            `json:"addressTreeHeight"`
	InclusionProofInputs    []common.InclusionProofInputsJSON `json:"inputCompressedAccounts"`
	NonInclusionProofInputs []NonInclusionProofInputsJSON     `json:"newAddresses"`
}

func (p *CombinedParameters) MarshalJSON() ([]byte, error) {
	inclusionJSON := p.InclusionParameters.CreateInclusionParametersJSON()
	nonInclusionJSON := p.NonInclusionParameters.CreateNonInclusionParametersJSON()

	combined := CombinedParametersJSON{
		CircuitType:             "combined",
		StateTreeHeight:         inclusionJSON.StateTreeHeight,      // v1 always uses height 26
		AddressTreeHeight:       nonInclusionJSON.AddressTreeHeight, // v1 always uses height 26
		InclusionProofInputs:    inclusionJSON.Inputs,
		NonInclusionProofInputs: nonInclusionJSON.Inputs,
	}
	return json.Marshal(combined)
}

func (p *CombinedParameters) UnmarshalJSON(data []byte) error {
	var params CombinedParametersJSON
	err := json.Unmarshal(data, &params)
	if err != nil {
		return err
	}

	if len(params.InclusionProofInputs) > 0 {
		inclusionJSON := InclusionParametersJSON{
			CircuitType:     "combined",
			StateTreeHeight: params.StateTreeHeight,
			Inputs:          params.InclusionProofInputs,
		}
		p.InclusionParameters = InclusionParameters{}
		err = p.InclusionParameters.UpdateWithJSON(inclusionJSON)
		if err != nil {
			return fmt.Errorf("failed to unmarshal inclusion parameters: %w", err)
		}
	}

	if len(params.NonInclusionProofInputs) > 0 {
		nonInclusionJSON := NonInclusionParametersJSON{
			CircuitType:       common.NonInclusionCircuitType,
			AddressTreeHeight: params.AddressTreeHeight,
			Inputs:            params.NonInclusionProofInputs,
		}
		p.NonInclusionParameters = NonInclusionParameters{}
		err = p.NonInclusionParameters.UpdateWithJSON(nonInclusionJSON, nil)
		if err != nil {
			return fmt.Errorf("failed to unmarshal non-inclusion parameters: %w", err)
		}
	}

	return nil
}
