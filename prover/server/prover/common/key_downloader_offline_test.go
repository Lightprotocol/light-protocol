package common

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDownloadKeyOfflineUsesExistingNonEmptyFile(t *testing.T) {
	keyPath := filepath.Join(t.TempDir(), "batch_update_32_500.key")
	if err := os.WriteFile(keyPath, []byte("existing-key"), 0o600); err != nil {
		t.Fatal(err)
	}

	config := DefaultDownloadConfig()
	config.AutoDownload = false
	config.BaseURL = "http://127.0.0.1:1"
	if err := DownloadKey(keyPath, config); err != nil {
		t.Fatalf("existing offline key rejected: %v", err)
	}
}

func TestDownloadKeyOfflineRejectsMissingOrEmptyFile(t *testing.T) {
	config := DefaultDownloadConfig()
	config.AutoDownload = false
	config.BaseURL = "http://127.0.0.1:1"

	missing := filepath.Join(t.TempDir(), "missing.key")
	if err := DownloadKey(missing, config); err == nil {
		t.Fatal("missing offline key unexpectedly accepted")
	}

	empty := filepath.Join(t.TempDir(), "empty.key")
	if err := os.WriteFile(empty, nil, 0o600); err != nil {
		t.Fatal(err)
	}
	if err := DownloadKey(empty, config); err == nil {
		t.Fatal("empty offline key unexpectedly accepted")
	}
}
