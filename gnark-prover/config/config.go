package config

import (
	"github.com/pelletier/go-toml/v2"
	"os"
)

type Config struct {
	Keys []string `toml:"keys"`
}

func (cfg *Config) HasKey(key string) bool {
	for _, k := range cfg.Keys {
		if k == key {
			return true
		}
	}
	return false
}

func ReadConfig(file string) (Config, error) {
	var cfg Config
	configFileData, err := os.ReadFile(file)
	if err != nil {
		return cfg, err
	}
	err = toml.Unmarshal(configFileData, &cfg)
	if err != nil {
		return cfg, err
	}
	return cfg, nil
}
