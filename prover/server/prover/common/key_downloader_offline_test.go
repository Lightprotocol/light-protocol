package common

import (
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
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

func TestDownloadKeyUsesBaseURLRootWithTrailingSlash(t *testing.T) {
	const filename = "test.key"
	const contents = "downloaded-key"
	const checksum = "1195e8d870f621c94ac378c38846612f075d0bd8da7fb727a873220ba6434a63"

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/CHECKSUM":
			_, _ = w.Write([]byte(checksum + "  " + filename + "\n"))
		case "/" + filename:
			_, _ = w.Write([]byte(contents))
		default:
			http.NotFound(w, r)
		}
	}))
	defer server.Close()

	config := DefaultDownloadConfig()
	config.BaseURL = server.URL + "/"
	config.MaxRetries = 1
	config.RetryDelay = time.Millisecond
	config.MaxRetryDelay = time.Millisecond

	keyPath := filepath.Join(t.TempDir(), filename)
	if err := DownloadKey(keyPath, config); err != nil {
		t.Fatalf("download failed: %v", err)
	}

	got, err := os.ReadFile(keyPath)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != contents {
		t.Fatalf("downloaded contents = %q, want %q", got, contents)
	}
}

func TestDownloadKeyRejectsMissingChecksumManifest(t *testing.T) {
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

	keyPath := filepath.Join(t.TempDir(), filename)
	err := DownloadKey(keyPath, config)
	if err == nil {
		t.Fatal("download without checksum manifest unexpectedly succeeded")
	}
	if got, want := err.Error(), "failed to load checksums: failed to download CHECKSUM file: HTTP 403"; got != want {
		t.Fatalf("unexpected error: %v", err)
	}
}
