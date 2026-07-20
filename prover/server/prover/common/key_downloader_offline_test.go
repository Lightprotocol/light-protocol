package common

import (
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
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

func TestDownloadKeyAllowsMissingChecksumManifest(t *testing.T) {
	const filename = "test.key"
	const contents = "downloaded-key"

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/CHECKSUM":
			w.WriteHeader(http.StatusForbidden)
		case "/" + filename:
			_, _ = w.Write([]byte(contents))
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	config := DefaultDownloadConfig()
	config.BaseURL = server.URL
	config.MaxRetries = 1
	config.RetryDelay = time.Millisecond
	config.MaxRetryDelay = time.Millisecond

	keyPath := filepath.Join(t.TempDir(), filename)
	if err := DownloadKey(keyPath, config); err != nil {
		t.Fatalf("manifest-less download failed: %v", err)
	}

	got, err := os.ReadFile(keyPath)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != contents {
		t.Fatalf("downloaded contents = %q, want %q", got, contents)
	}
}

func TestDownloadKeyCanRequireChecksumManifest(t *testing.T) {
	const filename = "test.key"

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/CHECKSUM" {
			w.WriteHeader(http.StatusForbidden)
			return
		}
		t.Fatal("key object requested without required checksum manifest")
	}))
	defer server.Close()

	config := DefaultDownloadConfig()
	config.BaseURL = server.URL
	config.MaxRetries = 1
	config.RequireChecksum = true

	keyPath := filepath.Join(t.TempDir(), filename)
	err := DownloadKey(keyPath, config)
	if err == nil {
		t.Fatal("download without required checksum manifest unexpectedly succeeded")
	}
	if !strings.Contains(err.Error(), "CHECKSUM file: HTTP 403") {
		t.Fatalf("unexpected error: %v", err)
	}
}
