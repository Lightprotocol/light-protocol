package prover

import (
	"encoding/json"
	"fmt"
	"math/big"
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

func TestBatchAddressTreeAppendCircuit(t *testing.T) {
	assert := test.NewAssert(t)

	t.Run("Basic single batch operations", func(t *testing.T) {
		testCases := []struct {
			name       string
			treeHeight uint32
			batchSize  uint32
		}{
			{"Small tree single element", 4, 1},
			// {"Small tree multiple elements", 4, 2},
			// {"Medium tree batch", 10, 5},
			// {"Large tree batch", 26, 10},
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				params := get_test_data_1_insert()
				circuit := createAddressCircuit(&params)
				witness := createAddressWitness(&params)

				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
				assert.NoError(err)
			})
		}
	})

	// t.Run("Small value gaps", func(t *testing.T) {
	// 	treeHeight := uint32(8)
	// 	batchSize := uint32(4)

	// 	var previousParams *BatchAddressTreeAppendParameters

	// 	// Process multiple batches and verify value gaps
	// 	for i := uint32(0); i < 32; i++ {
	// 		startIndex := i * batchSize
	// 		params := BuildTestBatchAddressAppend(treeHeight, batchSize, startIndex, previousParams, "")

	// 		// Verify minimum gaps between values
	// 		for j := uint32(0); j < params.BatchSize; j++ {
	// 			low := params.LowElements[j].Value
	// 			mid := params.LowElements[j].NextValue
	// 			high := params.NewElements[j].NextValue

	// 			diff1 := new(big.Int).Sub(mid, low)
	// 			diff2 := new(big.Int).Sub(high, mid)

	// 			minGap := big.NewInt(1000)
	// 			assert.True(diff1.Cmp(minGap) > 0, "Gap too small between low and mid values")
	// 			assert.True(diff2.Cmp(minGap) > 0, "Gap too small between mid and high values")
	// 		}

	// 		previousParams = params
	// 	}
	// })

	// t.Run("Near capacity handling", func(t *testing.T) {
	// 	treeHeight := uint32(8)
	// 	maxNodes := uint32(1 << treeHeight)
	// 	startIndex := maxNodes - 8 // Test very close to capacity

	// 	params := BuildTestBatchAddressAppend(treeHeight, 4, startIndex, nil, "")

	// 	// Verify value gaps
	// 	for i := uint32(0); i < params.BatchSize; i++ {
	// 		gapLow := new(big.Int).Sub(params.LowElements[i].NextValue, params.LowElements[i].Value)
	// 		gapHigh := new(big.Int).Sub(params.NewElements[i].NextValue, params.NewElements[i].Value)

	// 		minGap := new(big.Int).Exp(big.NewInt(2), big.NewInt(50), nil)
	// 		assert.True(gapLow.Cmp(minGap) > 0, "Low gap too small")
	// 		assert.True(gapHigh.Cmp(minGap) > 0, "High gap too small")
	// 	}
	// })

	// 	t.Run("Multiple sequential batches", func(t *testing.T) {
	// 		treeHeight := uint32(10)
	// 		batchSize := uint32(4)
	// 		numBatches := uint32(3)

	// 		var previousParams *BatchAddressTreeAppendParameters

	// 		for i := uint32(0); i < numBatches; i++ {
	// 			startIndex := i * batchSize
	// 			t.Run(fmt.Sprintf("Batch %d", i), func(t *testing.T) {
	// 				params := BuildTestBatchAddressAppend(
	// 					treeHeight,
	// 					batchSize,
	// 					startIndex,
	// 					previousParams,
	// 					"",
	// 				)

	// 				circuit := createAddressCircuit(params)
	// 				witness := createAddressWitness(params)
	// 				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
	// 				assert.NoError(err)

	// 				if previousParams != nil {
	// 					assert.Equal(params.OldRoot, previousParams.NewRoot)
	// 					assert.Equal(params.StartIndex, previousParams.StartIndex+previousParams.BatchSize)
	// 					assert.NotNil(params.Tree)
	// 				}

	// 				previousParams = params
	// 			})
	// 		}
	// 	})

	// 	t.Run("Fill tree completely", func(t *testing.T) {
	// 		treeHeight := uint32(8)
	// 		batchSize := uint32(4)
	// 		totalLeaves := uint32(1<<treeHeight) - 3

	// 		var previousParams *BatchAddressTreeAppendParameters

	// 		for startIndex := uint32(0); startIndex < totalLeaves; startIndex += batchSize {
	// 			t.Run(fmt.Sprintf("Batch starting at %d", startIndex), func(t *testing.T) {
	// 				remainingLeaves := totalLeaves - startIndex
	// 				currentBatchSize := batchSize
	// 				if remainingLeaves < batchSize {
	// 					currentBatchSize = remainingLeaves
	// 				}

	// 				params := BuildTestBatchAddressAppend(
	// 					treeHeight,
	// 					currentBatchSize,
	// 					startIndex,
	// 					previousParams,
	// 					"",
	// 				)

	// 				circuit := createAddressCircuit(params)
	// 				witness := createAddressWitness(params)

	// 				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
	// 				assert.NoError(err)

	// 				previousParams = params
	// 			})
	// 		}
	// 	})

	// 	t.Run("Element linking verification", func(t *testing.T) {
	// 		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

	// 		for i := uint32(0); i < params.BatchSize; i++ {
	// 			t.Run(fmt.Sprintf("Element pair %d", i), func(t *testing.T) {
	// 				if params.LowElements[i].NextValue.Cmp(params.NewElements[i].Value) != 0 {
	// 					t.Errorf("Low element next value (%s) doesn't match new element value (%s)",
	// 						params.LowElements[i].NextValue.String(),
	// 						params.NewElements[i].Value.String())
	// 				}

	// 				if params.LowElements[i].NextIndex != params.NewElements[i].Index {
	// 					t.Errorf("Low element next index (%d) doesn't match new element index (%d)",
	// 						params.LowElements[i].NextIndex,
	// 						params.NewElements[i].Index)
	// 				}

	// 				assert.Equal(len(params.LowElementProofs[i]), int(params.TreeHeight))
	// 				assert.Equal(len(params.NewElementProofs[i]), int(params.TreeHeight))
	// 			})
	// 		}
	// 	})

	// 	t.Run("Hash chain verification", func(t *testing.T) {
	// 		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

	// 		var leafHashesBI []*big.Int
	// 		for i := uint32(0); i < params.BatchSize; i++ {
	// 			lowLeafHash, err := merkletree.HashIndexedElement(&params.LowElements[i])
	// 			assert.NoError(err)
	// 			leafHashesBI = append(leafHashesBI, lowLeafHash)

	// 			newLeafHash, err := merkletree.HashIndexedElement(&params.NewElements[i])
	// 			assert.NoError(err)
	// 			leafHashesBI = append(leafHashesBI, newLeafHash)
	// 		}

	// 		calculatedHashChainBI := calculateHashChain(leafHashesBI, len(leafHashesBI))
	// 		assert.Equal(0, calculatedHashChainBI.Cmp(params.HashchainHash))
	// 	})

	// 	t.Run("Root verification", func(t *testing.T) {
	// 		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")

	// 		rootVal := params.Tree.Tree.Root.Value()
	// 		rootCopy := new(big.Int).Set(&rootVal) // Create a copy of the root value

	// 		assert.Equal(0, rootCopy.Cmp(params.NewRoot.(*big.Int)))
	// 	})

	// 	t.Run("Invalid cases", func(t *testing.T) {
	// 		testCases := []struct {
	// 			name        string
	// 			invalidCase string
	// 		}{
	// 			{"Invalid low element", "invalid_tree"},
	// 			{"Tree full", "tree_full"},
	// 			{"Value out of range", "invalid_range"},
	// 		}

	// 		for _, tc := range testCases {
	// 			t.Run(tc.name, func(t *testing.T) {
	// 				params := BuildTestBatchAddressAppend(26, 10, 0, nil, tc.invalidCase)
	// 				circuit := createAddressCircuit(params)
	// 				witness := createAddressWitness(params)

	// 				err := test.IsSolved(circuit, witness, ecc.BN254.ScalarField())
	// 				assert.Error(err)
	// 			})
	// 		}
	// 	})
	// }

	// func TestCreateAddressCircuit(t *testing.T) {
	// 	t.Run("Circuit creation", func(t *testing.T) {
	// 		params := BuildTestBatchAddressAppend(10, 2, 0, nil, "")
	// 		circuit := createAddressCircuit(params)

	// 		assert.Equal(t, params.BatchSize, circuit.BatchSize)
	// 		assert.Equal(t, params.TreeHeight, circuit.TreeHeight)
	// 		assert.Equal(t, int(params.BatchSize), len(circuit.LowElementProofs))
	// 		assert.Equal(t, int(params.BatchSize), len(circuit.NewElementProofs))

	//		for i := uint32(0); i < params.BatchSize; i++ {
	//			assert.Equal(t, int(params.TreeHeight), len(circuit.LowElementProofs[i]))
	//			assert.Equal(t, int(params.TreeHeight), len(circuit.NewElementProofs[i]))
	//		}
	//	})
}

// func TestHashChain(t *testing.T) {
// 	assert := test.NewAssert(t)

// 	t.Run("Empty chain", func(t *testing.T) {
// 		result := hashChain(0, []frontend.Variable{})
// 		assert.Equal(0, result.Cmp(big.NewInt(0)))
// 	})

// 	t.Run("Single element", func(t *testing.T) {
// 		input := big.NewInt(42)
// 		result := hashChain(1, []frontend.Variable{input})
// 		assert.Equal(0, result.Cmp(input))
// 	})

// 	t.Run("Multiple elements", func(t *testing.T) {
// 		inputs := []frontend.Variable{
// 			big.NewInt(1),
// 			big.NewInt(2),
// 			big.NewInt(3),
// 		}
// 		result := hashChain(len(inputs), inputs)
// 		assert.NotNil(result)
// 		assert.NotEqual(0, result.Cmp(big.NewInt(0)))
// 	})
// }

func createAddressCircuit(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	if params == nil {
		panic("params cannot be nil")
	}
	lowElementProofs := make([][]frontend.Variable, params.BatchSize)
	newElementProofs := make([][]frontend.Variable, params.BatchSize)

	for i := 0; i < int(params.BatchSize); i++ {
		lowElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
		newElementProofs[i] = make([]frontend.Variable, params.TreeHeight)
	}

	circuit := &BatchAddressTreeAppendCircuit{
		PublicInputHash: frontend.Variable(0),
		OldRoot:         frontend.Variable(0),
		NewRoot:         frontend.Variable(0),
		HashchainHash:   frontend.Variable(0),
		StartIndex:      frontend.Variable(0),

		LowElementValues:      make([]frontend.Variable, params.BatchSize),
		LowElementNextValues:  make([]frontend.Variable, params.BatchSize),
		LowElementNextIndices: make([]frontend.Variable, params.BatchSize),
		LowElementIndices:     make([]frontend.Variable, params.BatchSize),
		LowElementProofs:      lowElementProofs,

		NewElementValues: make([]frontend.Variable, params.BatchSize),
		NewElementProofs: newElementProofs,

		BatchSize:  params.BatchSize,
		TreeHeight: params.TreeHeight,
	}

	return circuit
}

func createAddressWitness(params *BatchAddressTreeAppendParameters) *BatchAddressTreeAppendCircuit {
	witness := createAddressCircuit(params)

	witness.PublicInputHash = frontend.Variable(params.PublicInputHash)
	witness.OldRoot = params.OldRoot
	witness.NewRoot = params.NewRoot
	witness.HashchainHash = frontend.Variable(params.HashchainHash)
	witness.StartIndex = frontend.Variable(params.StartIndex)
	fmt.Println("createAddressWitness params.BatchSize ", params.BatchSize)
	for i := uint32(0); i < params.BatchSize; i++ {
		witness.LowElementValues[i] = frontend.Variable(params.LowElementValues[i])
		witness.LowElementIndices[i] = frontend.Variable(params.LowElementIndices[i])
		witness.LowElementNextIndices[i] = frontend.Variable(params.LowElementNextIndices[i])
		witness.LowElementNextValues[i] = frontend.Variable(params.LowElementNextValues[i])

		witness.NewElementValues[i] = frontend.Variable(params.NewElementValues[i])
		witness.LowElementProofs[i] = make([]frontend.Variable, len(params.LowElementProofs[i]))
		witness.NewElementProofs[i] = make([]frontend.Variable, len(params.NewElementProofs[i]))

		for j := 0; j < len(params.LowElementProofs[i]); j++ {
			witness.LowElementProofs[i][j] = frontend.Variable(params.LowElementProofs[i][j])
		}
		for j := 0; j < len(params.NewElementProofs[i]); j++ {
			witness.NewElementProofs[i][j] = frontend.Variable(params.NewElementProofs[i][j])
		}

	}

	return witness
}

type JsonBatchAddressTreeAppendParameters struct {
	PublicInputHash       string     `json:"PublicInputHash"`
	OldRoot               string     `json:"OldRoot"`
	NewRoot               string     `json:"NewRoot"`
	HashchainHash         string     `json:"HashchainHash"`
	StartIndex            uint32     `json:"StartIndex"`
	LowElementValues      []string   `json:"LowElementValues"`
	LowElementIndices     []string   `json:"LowElementIndices"`
	LowElementNextIndices []string   `json:"LowElementNextIndices"`
	LowElementNextValues  []string   `json:"LowElementNextValues"`
	NewElementValues      []string   `json:"NewElementValues"`
	LowElementProofs      [][]string `json:"LowElementProofs"`
	NewElementProofs      [][]string `json:"NewElementProofs"`
	TreeHeight            string     `json:"TreeHeight"`
	BatchSize             string     `json:"BatchSize"`
}

func get_test_data_1_insert() BatchAddressTreeAppendParameters {
	jsonData := `{
		"PublicInputHash": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
		"OldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
		"NewRoot": "9360296153682585144477573401014216227476090504161963538926993860213587505773",
		"HashchainHash": "303229927723846428264111808645197890460298805508752674790354860916280465234",
		"StartIndex": 2,
		"LowElementValues": ["0"],
		"LowElementIndices": ["0"],
		"LowElementNextIndices": ["1"],
		"LowElementNextValues": ["452312848583266388373324160190187140051835877600158453279131187530910662655"],
		"NewElementValues": ["0"],
		"LowElementProofs": [
			["13859306649965657812382249699983066845935552967038026417581136538215435035637", 
			"15723694721673876141054887912220565198608814743306664888649577252769766605905", 
			"7423237065226347324353380772367382631490014989348495481811164164159255474657", 
			"11286972368698509976183087595462810875513684078608517520839298933882497716792"]
		],
		"NewElementProofs": [
			["0"],
			["8547627707011610151354520719421304546275272960684542609603067758776562440032"],
			["7423237065226347324353380772367382631490014989348495481811164164159255474657"],
			["11286972368698509976183087595462810875513684078608517520839298933882497716792"]
		],
		"TreeHeight": 4,
		"BatchSize": 1
	}`
	batchSize := 1
	params := BatchAddressTreeAppendParameters{
		PublicInputHash: big.NewInt(0),
		OldRoot:         big.NewInt(0),
		NewRoot:         big.NewInt(0),
		HashchainHash:   big.NewInt(0),
		StartIndex:      uint32(0),
		// Elements being modified or added
		LowElementValues:      make([]big.Int, batchSize),
		LowElementIndices:     make([]big.Int, batchSize),
		LowElementNextIndices: make([]big.Int, batchSize),
		LowElementNextValues:  make([]big.Int, batchSize),

		NewElementValues: make([]big.Int, batchSize),

		// Merkle proofs for verification
		LowElementProofs: make([][]big.Int, batchSize),
		NewElementProofs: make([][]big.Int, batchSize),

		// Tree configuration
		TreeHeight: 4,
		BatchSize:  1,
	}

	json.Unmarshal([]byte(jsonData), &params)
	return params
}

func get_test_data_2_insert() BatchAddressTreeAppendParameters {
	jsonData := `{
	"publicInputHash": "13832493800585898166114088680240082093515072733861191400485854249726904407056",
	"oldRoot": "4088321652280896297689618452575487697996974475167705659475507189280537614421",
	"newRoot": "19171761698158463304851719404211649373247489649232357264671218893697063910840",
	"hashchainHash": "13832493800585898166114088680240082093515072733861191400485854249726904407056",
	"startIndex": 0,
	"lowElementValues": ["0", "0"],
	"lowElementIndices": ["0", "0"],
	"lowElementNextIndices": ["1", "2"],
	"lowElementNextValues": [
	  "452312848583266388373324160190187140051835877600158453279131187530910662655",
	  "31"
	],
	"newElementValues": ["0", "18759147822983135752235477210489921219029732284985839751594385139872277119211"],
	"lowElementProofs": [
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"3796870957066466565085934353165460010672460214992313636808730505094443732049",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ],
	  [
		"13859306649965657812382249699983066845935552967038026417581136538215435035637",
		"9326699937298322758647210248953324788180879800245446112806190282402299353280",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ]
	],
	"newElementProofs": [
	  [
		"0",
		"20349398328338424766777768544456194850476422282930688296176215598270154437882",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ],
	  [
		"18759147822983135752235477210489921219029732284985839751594385139872277119211",
		"9756630233563453530006719845367697306884777538379147077278822652368465505916",
		"7423237065226347324353380772367382631490014989348495481811164164159255474657",
		"11286972368698509976183087595462810875513684078608517520839298933882497716792"
	  ]
	],
	"treeHeight": 4,
	"batchSize": 2
  }`
	batchSize := 1
	params := BatchAddressTreeAppendParameters{
		PublicInputHash: big.NewInt(0),
		OldRoot:         big.NewInt(0),
		NewRoot:         big.NewInt(0),
		HashchainHash:   big.NewInt(0),
		StartIndex:      uint32(0),
		// Elements being modified or added
		LowElementValues:      make([]big.Int, batchSize),
		LowElementIndices:     make([]big.Int, batchSize),
		LowElementNextIndices: make([]big.Int, batchSize),
		LowElementNextValues:  make([]big.Int, batchSize),

		NewElementValues: make([]big.Int, batchSize),

		// Merkle proofs for verification
		LowElementProofs: make([][]big.Int, batchSize),
		NewElementProofs: make([][]big.Int, batchSize),

		// Tree configuration
		TreeHeight: 4,
		BatchSize:  1,
	}

	json.Unmarshal([]byte(jsonData), &params)
	return params
}

// func hashChain(length int, inputs []frontend.Variable) *big.Int {
// 	if len(inputs) == 0 {
// 		return big.NewInt(0)
// 	}
// 	if len(inputs) == 1 {
// 		return inputs[0].(*big.Int)
// 	}

// 	hashChain := inputs[0].(*big.Int)
// 	for i := 1; i < length; i++ {
// 		hash, err := poseidon.Hash([]*big.Int{hashChain, inputs[i].(*big.Int)})
// 		if err != nil {
// 			panic(fmt.Sprintf("Failed to hash chain: %v", err))
// 		}
// 		hashChain = hash
// 	}
// 	return hashChain
// }

// type BatchAddressUpdateState struct {
// 	TreeRoot         big.Int
// 	LowElement       merkletree.IndexedElement
// 	NewElement       merkletree.IndexedElement
// 	LowElementProof  []big.Int
// 	NewElementProof  []big.Int
// 	CurrentTreeState *merkletree.IndexedMerkleTree
// }

// func BuildTestBatchAddressAppend(treeHeight uint32, batchSize uint32, startIndex uint32, previousParams *BatchAddressTreeAppendParameters, invalidCase string) *BatchAddressTreeAppendParameters {
// 	maxNodes := uint32(1 << treeHeight)

// 	// Early capacity checks
// 	if startIndex >= maxNodes {
// 		panic("Start index exceeds tree capacity")
// 	}

// 	// Calculate actual remaining capacity accounting for the need of two slots per operation
// 	remainingCapacity := maxNodes - startIndex
// 	if remainingCapacity < 1 { // Need at least 1 slot for proper linking
// 		panic("Insufficient tree capacity")
// 	}

// 	// Adjust batch size based on remaining capacity
// 	// We need one slot for each new element
// 	if batchSize > remainingCapacity {
// 		batchSize = remainingCapacity
// 	}

// 	// Further reduce batch size if we don't have enough slots
// 	if startIndex+batchSize > maxNodes {
// 		batchSize = maxNodes - startIndex
// 	}

// 	// Verify batch size is still valid
// 	if batchSize == 0 {
// 		panic("Batch size reduced to 0 due to capacity constraints")
// 	}

// 	// Minimum value gap to prevent small number issues
// 	minGap := new(big.Int).Exp(big.NewInt(2), big.NewInt(50), nil)

// 	var tree *merkletree.IndexedMerkleTree
// 	var oldRoot big.Int
// 	var lastBatchElement *merkletree.IndexedElement

// 	// // Special handling for invalid cases
// 	// switch invalidCase {
// 	// case "invalid_tree":
// 	// 	tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
// 	// 	tree.Init()
// 	// 	oldRoot := tree.Tree.Root.Value()
// 	// 	newRoot := oldRoot

// 	// 	params := BatchAddressTreeAppendParameters{
// 	// 		PublicInputHash:  big.NewInt(0),
// 	// 		OldRoot:          &oldRoot,
// 	// 		NewRoot:          &newRoot,
// 	// 		HashchainHash:    big.NewInt(0),
// 	// 		StartIndex:       startIndex,
// 	// 		OldLowElements:   make([]merkletree.IndexedElement, batchSize),
// 	// 		LowElements:      make([]merkletree.IndexedElement, batchSize),
// 	// 		NewElements:      make([]merkletree.IndexedElement, batchSize),
// 	// 		LowElementProofs: make([][]big.Int, batchSize),
// 	// 		NewElementProofs: make([][]big.Int, batchSize),
// 	// 		TreeHeight:       treeHeight,
// 	// 		BatchSize:        batchSize,
// 	// 		Tree:             tree,
// 	// 	}
// 	// 	for i := uint32(0); i < batchSize; i++ {
// 	// 		params.LowElementProofs[i] = make([]big.Int, treeHeight)
// 	// 		params.NewElementProofs[i] = make([]big.Int, treeHeight)

// 	// 		params.OldLowElements[i] = merkletree.IndexedElement{
// 	// 			Value:     big.NewInt(int64(i)),
// 	// 			NextValue: big.NewInt(int64(i + 1)),
// 	// 			NextIndex: maxNodes + 1, // Invalid next index
// 	// 			Index:     i,
// 	// 		}

// 	// 		params.LowElements[i] = params.OldLowElements[i]
// 	// 		params.NewElements[i] = params.OldLowElements[i]
// 	// 	}

// 	// 	return &params

// 	// case "tree_full":
// 	// 	tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
// 	// 	tree.Init()
// 	// 	oldRoot := tree.Tree.Root.Value()
// 	// 	newRoot := oldRoot

// 	// 	params := BatchAddressTreeAppendParameters{
// 	// 		PublicInputHash:  big.NewInt(0),
// 	// 		OldRoot:          &oldRoot,
// 	// 		NewRoot:          &newRoot,
// 	// 		HashchainHash:    big.NewInt(0),
// 	// 		StartIndex:       maxNodes - 1,
// 	// 		OldLowElements:   make([]merkletree.IndexedElement, 1),
// 	// 		LowElements:      make([]merkletree.IndexedElement, 1),
// 	// 		NewElements:      make([]merkletree.IndexedElement, 1),
// 	// 		LowElementProofs: make([][]big.Int, 1),
// 	// 		NewElementProofs: make([][]big.Int, 1),
// 	// 		TreeHeight:       treeHeight,
// 	// 		BatchSize:        1,
// 	// 		Tree:             tree,
// 	// 	}

// 	// 	params.LowElementProofs[0] = make([]big.Int, treeHeight)
// 	// 	params.NewElementProofs[0] = make([]big.Int, treeHeight)

// 	// 	return &params

// 	// case "invalid_range":
// 	// 	// Create tree with invalid range values
// 	// 	tree, _ := merkletree.NewIndexedMerkleTree(treeHeight)
// 	// 	tree.Init()
// 	// 	oldRoot := tree.Tree.Root.Value()
// 	// 	newRoot := oldRoot

// 	// 	params := BatchAddressTreeAppendParameters{
// 	// 		PublicInputHash:  big.NewInt(0),
// 	// 		OldRoot:          &oldRoot,
// 	// 		NewRoot:          &newRoot,
// 	// 		HashchainHash:    big.NewInt(0),
// 	// 		StartIndex:       maxNodes * 2,
// 	// 		OldLowElements:   make([]merkletree.IndexedElement, batchSize),
// 	// 		LowElements:      make([]merkletree.IndexedElement, batchSize),
// 	// 		NewElements:      make([]merkletree.IndexedElement, batchSize),
// 	// 		LowElementProofs: make([][]big.Int, batchSize),
// 	// 		NewElementProofs: make([][]big.Int, batchSize),
// 	// 		TreeHeight:       treeHeight,
// 	// 		BatchSize:        batchSize,
// 	// 		Tree:             tree,
// 	// 	}

// 	// 	for i := uint32(0); i < batchSize; i++ {
// 	// 		params.LowElementProofs[i] = make([]big.Int, treeHeight)
// 	// 		params.NewElementProofs[i] = make([]big.Int, treeHeight)

// 	// 		params.OldLowElements[i] = merkletree.IndexedElement{
// 	// 			Value:     big.NewInt(int64(maxNodes*2 + i)),
// 	// 			NextValue: big.NewInt(int64(maxNodes*2 + i + 1)),
// 	// 			NextIndex: i,
// 	// 			Index:     i,
// 	// 		}

// 	// 		params.LowElements[i] = params.OldLowElements[i]
// 	// 		params.NewElements[i] = params.OldLowElements[i]
// 	// 	}

// 	// 	return &params
// 	// }

// 	if previousParams != nil {
// 		tree = previousParams.Tree.DeepCopy()
// 		oldRoot = *previousParams.NewRoot.(*big.Int)
// 		if len(previousParams.NewElements) > 0 {
// 			lastElement := previousParams.NewElements[previousParams.BatchSize-1]
// 			lastBatchElement = &lastElement
// 		}
// 	} else {
// 		var err error
// 		tree, err = merkletree.NewIndexedMerkleTree(treeHeight)
// 		if err != nil {
// 			panic(fmt.Sprintf("Failed to create indexed merkle tree: %v", err))
// 		}
// 		err = tree.Init()
// 		if err != nil {
// 			panic(fmt.Sprintf("Failed to initialize indexed merkle tree: %v", err))
// 		}
// 		oldRoot = tree.Tree.Root.Value()
// 	}

// 	// updateStates := make([]BatchAddressUpdateState, batchSize)
// 	// oldLowElements := make([]merkletree.IndexedElement, batchSize)
// 	lowElements := make([]merkletree.IndexedElement, batchSize)
// 	newElements := make([]big.Int, batchSize)

// 	for i := uint32(0); i < batchSize; i++ {
// 		// Find low element
// 		var lowElement *merkletree.IndexedElement
// 		if i == 0 && lastBatchElement != nil {
// 			lowElement = lastBatchElement
// 		} else {
// 			lowElementIndex := tree.IndexArray.FindLowElementIndex(
// 				big.NewInt(int64(startIndex + i)),
// 			)
// 			lowElement = tree.IndexArray.Get(lowElementIndex)
// 			if lowElement == nil {
// 				batchSize = i
// 				break
// 			}
// 		}

// 		nextElementIndex := uint32(len(tree.IndexArray.Elements))
// 		if nextElementIndex >= maxNodes {
// 			batchSize = i
// 			break
// 		}

// 		oldLowElements[i] = *lowElement

// 		// Calculate available space
// 		diff := new(big.Int).Sub(lowElement.NextValue, lowElement.Value)

// 		// Ensure minimum gap
// 		if diff.Cmp(new(big.Int).Mul(minGap, big.NewInt(2))) <= 0 {
// 			batchSize = i
// 			break
// 		}

// 		// Calculate new value to maintain large gaps
// 		thirdsPoint := new(big.Int).Div(diff, big.NewInt(3))
// 		newValue := new(big.Int).Add(lowElement.Value, thirdsPoint)

// 		var nextValue *big.Int
// 		var nextIndex uint32

// 		if i == batchSize-1 && startIndex+batchSize < maxNodes {
// 			nextValue = lowElement.NextValue
// 			nextIndex = lowElement.NextIndex
// 		} else {
// 			nextValue = lowElement.NextValue
// 			nextIndex = nextElementIndex + 1
// 		}

// 		// Safety check for value ordering
// 		if newValue.Cmp(lowElement.Value) <= 0 || newValue.Cmp(nextValue) >= 0 {
// 			batchSize = i
// 			break
// 		}

// 		// Create elements with bounds checking
// 		if nextElementIndex < maxNodes {
// 			updatedLowElement := merkletree.IndexedElement{
// 				Value:     lowElement.Value,
// 				NextValue: newValue,
// 				NextIndex: nextElementIndex,
// 				Index:     lowElement.Index,
// 			}

// 			newElement := merkletree.IndexedElement{
// 				Value:     newValue,
// 				NextValue: nextValue,
// 				NextIndex: nextIndex,
// 				Index:     nextElementIndex,
// 			}

// 			lowElements[i] = updatedLowElement
// 			newElements[i] = newElement

// 			// Update tree state
// 			lowLeafHash, err := merkletree.HashIndexedElement(&updatedLowElement)
// 			if err != nil {
// 				panic(fmt.Sprintf("Failed to hash low leaf: %v", err))
// 			}

// 			lowProof := tree.Tree.GenerateProof(int(lowElement.Index))
// 			tree.Tree.Update(int(lowElement.Index), *lowLeafHash)

// 			newLeafHash, err := merkletree.HashIndexedElement(&newElement)
// 			if err != nil {
// 				panic(fmt.Sprintf("Failed to hash new leaf: %v", err))
// 			}

// 			newProof := tree.Tree.GenerateProof(int(nextElementIndex))
// 			tree.Tree.Update(int(nextElementIndex), *newLeafHash)

// 			updateStates[i] = BatchAddressUpdateState{
// 				TreeRoot:        tree.Tree.Root.Value(),
// 				LowElement:      updatedLowElement,
// 				NewElement:      newElement,
// 				LowElementProof: lowProof,
// 				NewElementProof: newProof,
// 			}

// 			tree.IndexArray.Elements[lowElement.Index] = updatedLowElement
// 			if int(nextElementIndex) >= len(tree.IndexArray.Elements) {
// 				tree.IndexArray.Elements = append(tree.IndexArray.Elements, newElement)
// 			} else {
// 				tree.IndexArray.Elements[nextElementIndex] = newElement
// 			}
// 			tree.IndexArray.CurrentNodeIndex = nextElementIndex
// 		} else {
// 			batchSize = i
// 			break
// 		}
// 	}

// 	// Adjust arrays to actual batch size
// 	if batchSize < uint32(len(oldLowElements)) {
// 		oldLowElements = oldLowElements[:batchSize]
// 		lowElements = lowElements[:batchSize]
// 		newElements = newElements[:batchSize]
// 		updateStates = updateStates[:batchSize]
// 	}

// 	newRoot := tree.Tree.Root.Value()
// 	var leafHashes []frontend.Variable
// 	for _, state := range updateStates {
// 		lowLeafHash, err := merkletree.HashIndexedElement(&state.LowElement)
// 		if err != nil {
// 			panic(err)
// 		}
// 		leafHashes = append(leafHashes, lowLeafHash)

// 		newLeafHash, err := merkletree.HashIndexedElement(&state.NewElement)
// 		if err != nil {
// 			panic(err)
// 		}
// 		leafHashes = append(leafHashes, newLeafHash)
// 	}

// 	leafHashChain := hashChain(len(leafHashes), leafHashes)

// 	publicInputHash := calculateHashChain([]*big.Int{
// 		&oldRoot,
// 		&newRoot,
// 		leafHashChain,
// 		big.NewInt(int64(startIndex)),
// 	}, 4)

// 	params := BatchAddressTreeAppendParameters{
// 		PublicInputHash:  publicInputHash,
// 		OldRoot:          &oldRoot,
// 		NewRoot:          &newRoot,
// 		HashchainHash:    leafHashChain,
// 		StartIndex:       startIndex,
// 		LowElements:      lowElements,
// 		NewElementValues: ,
// 		LowElementProofs: make([][]big.Int, batchSize),
// 		NewElementProofs: make([][]big.Int, batchSize),
// 		TreeHeight:       treeHeight,
// 		BatchSize:        batchSize,
// 		Tree:             tree,
// 	}

// 	for i := uint32(0); i < batchSize; i++ {
// 		params.LowElementProofs[i] = make([]big.Int, len(updateStates[i].LowElementProof))
// 		copy(params.LowElementProofs[i], updateStates[i].LowElementProof)

// 		params.NewElementProofs[i] = make([]big.Int, len(updateStates[i].NewElementProof))
// 		copy(params.NewElementProofs[i], updateStates[i].NewElementProof)
// 	}

// 	return &params
// }
