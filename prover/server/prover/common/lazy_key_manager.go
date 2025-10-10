package common

import (
	"fmt"
	"light/light-prover/logging"
	"strings"
	"sync"
)

type LazyKeyManager struct {
	mu                sync.RWMutex
	merkleSystems     map[string]*MerkleProofSystem
	batchSystems      map[string]*BatchProofSystem
	keysDir           string
	downloadConfig    *DownloadConfig
	loadingInProgress map[string]chan struct{}
}

func NewLazyKeyManager(keysDir string, downloadConfig *DownloadConfig) *LazyKeyManager {
	if downloadConfig == nil {
		downloadConfig = DefaultDownloadConfig()
	}
	return &LazyKeyManager{
		merkleSystems:     make(map[string]*MerkleProofSystem),
		batchSystems:      make(map[string]*BatchProofSystem),
		keysDir:           keysDir,
		downloadConfig:    downloadConfig,
		loadingInProgress: make(map[string]chan struct{}),
	}
}

func (m *LazyKeyManager) GetMerkleSystem(
	inclusionTreeHeight uint32,
	inclusionCompressedAccounts uint32,
	nonInclusionTreeHeight uint32,
	nonInclusionCompressedAccounts uint32,
	version uint32,
) (*MerkleProofSystem, error) {
	var key string
	if inclusionCompressedAccounts > 0 && nonInclusionCompressedAccounts > 0 {
		key = fmt.Sprintf("comb_%d_%d_%d_%d_v%d", inclusionTreeHeight, inclusionCompressedAccounts, nonInclusionTreeHeight, nonInclusionCompressedAccounts, version)
	} else if inclusionCompressedAccounts > 0 {
		key = fmt.Sprintf("inc_%d_%d_v%d", inclusionTreeHeight, inclusionCompressedAccounts, version)
	} else if nonInclusionCompressedAccounts > 0 {
		key = fmt.Sprintf("non_%d_%d_v%d", nonInclusionTreeHeight, nonInclusionCompressedAccounts, version)
	} else {
		return nil, fmt.Errorf("invalid parameters: must specify either inclusion or non-inclusion accounts")
	}

	m.mu.RLock()
	if ps, exists := m.merkleSystems[key]; exists {
		m.mu.RUnlock()
		logging.Logger().Debug().
			Str("key", key).
			Msg("Found cached MerkleProofSystem")
		return ps, nil
	}
	m.mu.RUnlock()

	return m.loadMerkleSystem(key, inclusionTreeHeight, inclusionCompressedAccounts, nonInclusionTreeHeight, nonInclusionCompressedAccounts, version)
}

func (m *LazyKeyManager) GetBatchSystem(circuitType CircuitType, treeHeight uint32, batchSize uint32) (*BatchProofSystem, error) {
	key := fmt.Sprintf("%s_%d_%d", circuitType, treeHeight, batchSize)

	m.mu.RLock()
	if ps, exists := m.batchSystems[key]; exists {
		m.mu.RUnlock()
		logging.Logger().Debug().
			Str("key", key).
			Msg("Found cached BatchProofSystem")
		return ps, nil
	}
	m.mu.RUnlock()

	return m.loadBatchSystem(key, circuitType, treeHeight, batchSize)
}

func (m *LazyKeyManager) loadMerkleSystem(
	key string,
	inclusionTreeHeight uint32,
	inclusionCompressedAccounts uint32,
	nonInclusionTreeHeight uint32,
	nonInclusionCompressedAccounts uint32,
	version uint32,
) (*MerkleProofSystem, error) {
	loadChan := m.acquireLoadingLock(key)
	if loadChan == nil {
		m.waitForLoading(key)
		m.mu.RLock()
		ps, exists := m.merkleSystems[key]
		m.mu.RUnlock()
		if exists {
			return ps, nil
		}
		return nil, fmt.Errorf("loading completed but system not found in cache")
	}
	defer m.releaseLoadingLock(key, loadChan)

	keyPath := m.determineMerkleKeyPath(inclusionTreeHeight, inclusionCompressedAccounts, nonInclusionTreeHeight, nonInclusionCompressedAccounts, version)
	if keyPath == "" {
		return nil, fmt.Errorf("no key file mapping for parameters: inc(%d,%d) non(%d,%d) v%d",
			inclusionTreeHeight, inclusionCompressedAccounts, nonInclusionTreeHeight, nonInclusionCompressedAccounts, version)
	}

	logging.Logger().Info().
		Str("key_path", keyPath).
		Str("cache_key", key).
		Msg("Loading MerkleProofSystem")

	if err := DownloadKey(keyPath, m.downloadConfig); err != nil {
		return nil, fmt.Errorf("failed to download key %s: %w", keyPath, err)
	}

	system, err := ReadSystemFromFile(keyPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load key %s: %w", keyPath, err)
	}

	ps, ok := system.(*MerkleProofSystem)
	if !ok {
		return nil, fmt.Errorf("expected MerkleProofSystem but got different type")
	}

	m.mu.Lock()
	m.merkleSystems[key] = ps
	m.mu.Unlock()

	logging.Logger().Info().
		Str("cache_key", key).
		Uint32("inc_height", ps.InclusionTreeHeight).
		Uint32("inc_accounts", ps.InclusionNumberOfCompressedAccounts).
		Uint32("non_height", ps.NonInclusionTreeHeight).
		Uint32("non_accounts", ps.NonInclusionNumberOfCompressedAccounts).
		Msg("MerkleProofSystem loaded and cached successfully")

	return ps, nil
}

func (m *LazyKeyManager) loadBatchSystem(key string, circuitType CircuitType, treeHeight uint32, batchSize uint32) (*BatchProofSystem, error) {
	loadChan := m.acquireLoadingLock(key)
	if loadChan == nil {
		m.waitForLoading(key)
		m.mu.RLock()
		ps, exists := m.batchSystems[key]
		m.mu.RUnlock()
		if exists {
			return ps, nil
		}
		return nil, fmt.Errorf("loading completed but system not found in cache")
	}
	defer m.releaseLoadingLock(key, loadChan)

	keyPath := m.determineBatchKeyPath(circuitType, treeHeight, batchSize)
	if keyPath == "" {
		return nil, fmt.Errorf("no key file mapping for %s with height %d and batch size %d", circuitType, treeHeight, batchSize)
	}

	logging.Logger().Info().
		Str("key_path", keyPath).
		Str("cache_key", key).
		Msg("Loading BatchProofSystem")

	if err := DownloadKey(keyPath, m.downloadConfig); err != nil {
		return nil, fmt.Errorf("failed to download key %s: %w", keyPath, err)
	}

	system, err := ReadSystemFromFile(keyPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load key %s: %w", keyPath, err)
	}

	ps, ok := system.(*BatchProofSystem)
	if !ok {
		return nil, fmt.Errorf("expected BatchProofSystem but got different type")
	}

	m.mu.Lock()
	m.batchSystems[key] = ps
	m.mu.Unlock()

	logging.Logger().Info().
		Str("cache_key", key).
		Uint32("tree_height", ps.TreeHeight).
		Uint32("batch_size", ps.BatchSize).
		Str("circuit_type", string(ps.CircuitType)).
		Msg("BatchProofSystem loaded and cached successfully")

	return ps, nil
}

func (m *LazyKeyManager) acquireLoadingLock(key string) chan struct{} {
	m.mu.Lock()
	defer m.mu.Unlock()

	if _, loading := m.loadingInProgress[key]; loading {
		return nil
	}

	ch := make(chan struct{})
	m.loadingInProgress[key] = ch
	return ch
}

func (m *LazyKeyManager) waitForLoading(key string) {
	m.mu.RLock()
	ch := m.loadingInProgress[key]
	m.mu.RUnlock()

	if ch != nil {
		<-ch
	}
}

func (m *LazyKeyManager) releaseLoadingLock(key string, ch chan struct{}) {
	m.mu.Lock()
	delete(m.loadingInProgress, key)
	m.mu.Unlock()
	close(ch)
}

func (m *LazyKeyManager) determineMerkleKeyPath(
	inclusionTreeHeight uint32,
	inclusionCompressedAccounts uint32,
	nonInclusionTreeHeight uint32,
	nonInclusionCompressedAccounts uint32,
	version uint32,
) string {
	if inclusionCompressedAccounts > 0 && nonInclusionCompressedAccounts > 0 {
		if version == 1 && inclusionTreeHeight == 26 && nonInclusionTreeHeight == 26 {
			return fmt.Sprintf("%sv1_combined_26_26_%d_%d.key", m.keysDir, inclusionCompressedAccounts, nonInclusionCompressedAccounts)
		} else if version == 2 && inclusionTreeHeight == 32 && nonInclusionTreeHeight == 40 {
			return fmt.Sprintf("%sv2_combined_32_40_%d_%d.key", m.keysDir, inclusionCompressedAccounts, nonInclusionCompressedAccounts)
		}
	} else if inclusionCompressedAccounts > 0 {
		if version == 1 && inclusionTreeHeight == 26 {
			return fmt.Sprintf("%sv1_inclusion_26_%d.key", m.keysDir, inclusionCompressedAccounts)
		} else if version == 2 && inclusionTreeHeight == 32 {
			return fmt.Sprintf("%sv2_inclusion_32_%d.key", m.keysDir, inclusionCompressedAccounts)
		}
	} else if nonInclusionCompressedAccounts > 0 {
		if version == 1 && nonInclusionTreeHeight == 26 {
			return fmt.Sprintf("%sv1_non-inclusion_26_%d.key", m.keysDir, nonInclusionCompressedAccounts)
		} else if version == 2 && nonInclusionTreeHeight == 40 {
			return fmt.Sprintf("%sv2_non-inclusion_40_%d.key", m.keysDir, nonInclusionCompressedAccounts)
		}
	}

	return ""
}

func (m *LazyKeyManager) determineBatchKeyPath(circuitType CircuitType, treeHeight uint32, batchSize uint32) string {
	switch circuitType {
	case BatchAppendCircuitType:
		if treeHeight == 32 && batchSize == 500 {
			return fmt.Sprintf("%sbatch_append_32_500.key", m.keysDir)
		} else if treeHeight == 32 && batchSize == 10 {
			return fmt.Sprintf("%sbatch_append_32_10.key", m.keysDir)
		}
	case BatchUpdateCircuitType:
		if treeHeight == 32 && batchSize == 500 {
			return fmt.Sprintf("%sbatch_update_32_500.key", m.keysDir)
		} else if treeHeight == 32 && batchSize == 10 {
			return fmt.Sprintf("%sbatch_update_32_10.key", m.keysDir)
		}
	case BatchAddressAppendCircuitType:
		if treeHeight == 40 && batchSize == 250 {
			return fmt.Sprintf("%sbatch_address-append_40_250.key", m.keysDir)
		} else if treeHeight == 40 && batchSize == 10 {
			return fmt.Sprintf("%sbatch_address-append_40_10.key", m.keysDir)
		}
	}

	return ""
}

func (m *LazyKeyManager) GetStats() map[string]interface{} {
	m.mu.RLock()
	defer m.mu.RUnlock()

	return map[string]interface{}{
		"merkle_systems_loaded": len(m.merkleSystems),
		"batch_systems_loaded":  len(m.batchSystems),
		"keys_loading":          len(m.loadingInProgress),
	}
}

func (m *LazyKeyManager) PreloadForRunMode(runMode RunMode) error {
	logging.Logger().Info().
		Str("run_mode", string(runMode)).
		Msg("Preloading keys for run mode")

	keys := GetKeys(m.keysDir, runMode, nil)
	return m.preloadKeys(keys)
}

func (m *LazyKeyManager) PreloadAll() error {
	logging.Logger().Info().Msg("Preloading all keys")

	allKeys := make(map[string]bool)
	runModes := []RunMode{Full, FullTest}
	for _, runMode := range runModes {
		keys := GetKeys(m.keysDir, runMode, nil)
		for _, key := range keys {
			allKeys[key] = true
		}
	}

	keySlice := make([]string, 0, len(allKeys))
	for key := range allKeys {
		keySlice = append(keySlice, key)
	}

	return m.preloadKeys(keySlice)
}

func (m *LazyKeyManager) PreloadCircuits(circuits []string) error {
	logging.Logger().Info().
		Strs("circuits", circuits).
		Msg("Preloading keys for circuits")

	var keyPaths []string
	seen := make(map[string]bool)

	for _, circuit := range circuits {
		if specificPath := m.tryParseSpecificConfig(circuit); specificPath != "" {
			if !seen[specificPath] {
				keyPaths = append(keyPaths, specificPath)
				seen[specificPath] = true
			}
			continue
		}

		circuitKeys := GetKeys(m.keysDir, "", []string{circuit})
		for _, key := range circuitKeys {
			if !seen[key] {
				keyPaths = append(keyPaths, key)
				seen[key] = true
			}
		}
	}

	return m.preloadKeys(keyPaths)
}

func (m *LazyKeyManager) tryParseSpecificConfig(config string) string {
	if strings.HasPrefix(config, "batch_") ||
		strings.HasPrefix(config, "v1_") ||
		strings.HasPrefix(config, "v2_") {
		return fmt.Sprintf("%s%s.key", m.keysDir, config)
	}
	return ""
}

func (m *LazyKeyManager) preloadKeys(keyPaths []string) error {
	if len(keyPaths) == 0 {
		logging.Logger().Info().Msg("No keys to preload")
		return nil
	}

	logging.Logger().Info().
		Int("count", len(keyPaths)).
		Msg("Starting to preload keys")

	for i, keyPath := range keyPaths {
		logging.Logger().Info().
			Int("current", i+1).
			Int("total", len(keyPaths)).
			Str("key_path", keyPath).
			Msg("Preloading key")

		if err := DownloadKey(keyPath, m.downloadConfig); err != nil {
			return fmt.Errorf("failed to download key %s: %w", keyPath, err)
		}

		system, err := ReadSystemFromFile(keyPath)
		if err != nil {
			return fmt.Errorf("failed to load key %s: %w", keyPath, err)
		}

		if err := m.cacheSystem(system); err != nil {
			return fmt.Errorf("failed to cache key %s: %w", keyPath, err)
		}
	}

	logging.Logger().Info().
		Int("count", len(keyPaths)).
		Msg("Successfully preloaded all keys")

	return nil
}

func (m *LazyKeyManager) cacheSystem(system interface{}) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	switch ps := system.(type) {
	case *MerkleProofSystem:
		var key string
		if ps.InclusionNumberOfCompressedAccounts > 0 && ps.NonInclusionNumberOfCompressedAccounts > 0 {
			key = fmt.Sprintf("comb_%d_%d_%d_%d_v%d",
				ps.InclusionTreeHeight,
				ps.InclusionNumberOfCompressedAccounts,
				ps.NonInclusionTreeHeight,
				ps.NonInclusionNumberOfCompressedAccounts,
				ps.Version)
		} else if ps.InclusionNumberOfCompressedAccounts > 0 {
			key = fmt.Sprintf("inc_%d_%d_v%d",
				ps.InclusionTreeHeight,
				ps.InclusionNumberOfCompressedAccounts,
				ps.Version)
		} else if ps.NonInclusionNumberOfCompressedAccounts > 0 {
			key = fmt.Sprintf("non_%d_%d_v%d",
				ps.NonInclusionTreeHeight,
				ps.NonInclusionNumberOfCompressedAccounts,
				ps.Version)
		} else {
			return fmt.Errorf("invalid MerkleProofSystem: no compressed accounts specified")
		}

		m.merkleSystems[key] = ps
		logging.Logger().Debug().
			Str("cache_key", key).
			Msg("Cached MerkleProofSystem")

	case *BatchProofSystem:
		key := fmt.Sprintf("%s_%d_%d", ps.CircuitType, ps.TreeHeight, ps.BatchSize)
		m.batchSystems[key] = ps
		logging.Logger().Debug().
			Str("cache_key", key).
			Msg("Cached BatchProofSystem")

	default:
		return fmt.Errorf("unknown system type: %T", system)
	}

	return nil
}
