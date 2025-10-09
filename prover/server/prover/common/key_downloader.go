package common

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"light/light-prover/logging"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const (
	DefaultBaseURL       = "https://storage.googleapis.com/light-protocol-proving-keys/proving-keys-06-10-25"
	DefaultMaxRetries    = 10
	DefaultRetryDelay    = 5 * time.Second
	DefaultMaxRetryDelay = 5 * time.Minute
)

type DownloadConfig struct {
	BaseURL       string
	MaxRetries    int
	RetryDelay    time.Duration
	MaxRetryDelay time.Duration
	AutoDownload  bool
}

func DefaultDownloadConfig() *DownloadConfig {
	return &DownloadConfig{
		BaseURL:       DefaultBaseURL,
		MaxRetries:    DefaultMaxRetries,
		RetryDelay:    DefaultRetryDelay,
		MaxRetryDelay: DefaultMaxRetryDelay,
		AutoDownload:  true,
	}
}

type checksumCache struct {
	checksums map[string]string
	loaded    bool
}

var globalChecksumCache = &checksumCache{
	checksums: make(map[string]string),
	loaded:    false,
}

func downloadChecksum(config *DownloadConfig) error {
	if globalChecksumCache.loaded {
		return nil
	}

	checksumURL := config.BaseURL + "/CHECKSUM"
	logging.Logger().Info().
		Str("url", checksumURL).
		Msg("Downloading CHECKSUM file")

	resp, err := http.Get(checksumURL)
	if err != nil {
		return fmt.Errorf("failed to download CHECKSUM file: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to download CHECKSUM file: HTTP %d", resp.StatusCode)
	}

	content, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read CHECKSUM file: %w", err)
	}

	// Parse CHECKSUM file (format: "checksum  filename")
	lines := strings.Split(string(content), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.Fields(line)
		if len(parts) >= 2 {
			checksum := parts[0]
			filename := parts[1]
			globalChecksumCache.checksums[filename] = checksum
		}
	}

	globalChecksumCache.loaded = true
	logging.Logger().Info().
		Int("count", len(globalChecksumCache.checksums)).
		Msg("Loaded checksums")

	return nil
}

func verifyChecksum(filepath string, expectedChecksum string) (bool, error) {
	file, err := os.Open(filepath)
	if err != nil {
		return false, err
	}
	defer file.Close()

	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return false, err
	}

	actualChecksum := hex.EncodeToString(hash.Sum(nil))
	return actualChecksum == expectedChecksum, nil
}

func calculateBackoff(attempt int, initialDelay, maxDelay time.Duration) time.Duration {
	delay := initialDelay * time.Duration(1<<uint(attempt-1))
	if delay > maxDelay {
		return maxDelay
	}
	return delay
}

func downloadFileWithResume(url, outputPath string, config *DownloadConfig) error {
	tempPath := outputPath + ".tmp"

	for attempt := 1; attempt <= config.MaxRetries; attempt++ {
		var existingSize int64 = 0
		if fileInfo, err := os.Stat(tempPath); err == nil {
			existingSize = fileInfo.Size()
		}

		req, err := http.NewRequest("GET", url, nil)
		if err != nil {
			return fmt.Errorf("failed to create request: %w", err)
		}

		if existingSize > 0 {
			req.Header.Set("Range", fmt.Sprintf("bytes=%d-", existingSize))
			logging.Logger().Info().
				Str("url", url).
				Int64("resume_from", existingSize).
				Int("attempt", attempt).
				Int("max_retries", config.MaxRetries).
				Msg("Resuming download")
		} else {
			logging.Logger().Info().
				Str("url", url).
				Int("attempt", attempt).
				Int("max_retries", config.MaxRetries).
				Msg("Starting download")
		}

		client := &http.Client{
			Timeout: 60 * time.Minute,
		}
		resp, err := client.Do(req)
		if err != nil {
			if attempt < config.MaxRetries {
				delay := calculateBackoff(attempt, config.RetryDelay, config.MaxRetryDelay)
				logging.Logger().Warn().
					Err(err).
					Dur("retry_delay", delay).
					Msg("Download failed, retrying")
				time.Sleep(delay)
				continue
			}
			return fmt.Errorf("failed to download after %d attempts: %w", config.MaxRetries, err)
		}

		// Check response status
		if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusPartialContent {
			resp.Body.Close()
			if attempt < config.MaxRetries {
				delay := calculateBackoff(attempt, config.RetryDelay, config.MaxRetryDelay)
				logging.Logger().Warn().
					Int("status_code", resp.StatusCode).
					Dur("retry_delay", delay).
					Msg("Unexpected status code, retrying")
				time.Sleep(delay)
				continue
			}
			return fmt.Errorf("unexpected status code: %d", resp.StatusCode)
		}

		var file *os.File
		if existingSize > 0 && resp.StatusCode == http.StatusPartialContent {
			file, err = os.OpenFile(tempPath, os.O_APPEND|os.O_WRONLY, 0644)
		} else {
			file, err = os.Create(tempPath)
			existingSize = 0
		}
		if err != nil {
			resp.Body.Close()
			return fmt.Errorf("failed to open file: %w", err)
		}

		totalSize := existingSize + resp.ContentLength
		downloadedBytes := existingSize
		lastLogTime := time.Now()
		logInterval := 5 * time.Second

		buffer := make([]byte, 32*1024)
		for {
			n, err := resp.Body.Read(buffer)
			if n > 0 {
				if _, writeErr := file.Write(buffer[:n]); writeErr != nil {
					file.Close()
					resp.Body.Close()
					return fmt.Errorf("failed to write to file: %w", writeErr)
				}
				downloadedBytes += int64(n)

				if time.Since(lastLogTime) >= logInterval {
					if totalSize > 0 {
						progress := float64(downloadedBytes) / float64(totalSize) * 100
						logging.Logger().Info().
							Int64("downloaded", downloadedBytes).
							Int64("total", totalSize).
							Float64("progress", progress).
							Msg("Download progress")
					}
					lastLogTime = time.Now()
				}
			}
			if err == io.EOF {
				break
			}
			if err != nil {
				file.Close()
				resp.Body.Close()
				if attempt < config.MaxRetries {
					delay := calculateBackoff(attempt, config.RetryDelay, config.MaxRetryDelay)
					logging.Logger().Warn().
						Err(err).
						Dur("retry_delay", delay).
						Msg("Download interrupted, retrying")
					time.Sleep(delay)
					continue
				}
				return fmt.Errorf("download failed: %w", err)
			}
		}

		file.Close()
		resp.Body.Close()

		if err := os.Rename(tempPath, outputPath); err != nil {
			return fmt.Errorf("failed to rename temp file: %w", err)
		}

		logging.Logger().Info().
			Str("file", filepath.Base(outputPath)).
			Int64("size", downloadedBytes).
			Msg("Download completed successfully")

		return nil
	}

	return fmt.Errorf("failed to download after %d attempts", config.MaxRetries)
}

func DownloadKey(keyPath string, config *DownloadConfig) error {
	filename := filepath.Base(keyPath)

	if err := downloadChecksum(config); err != nil {
		return fmt.Errorf("failed to load checksums: %w", err)
	}

	expectedChecksum, exists := globalChecksumCache.checksums[filename]
	if !exists {
		return fmt.Errorf("no checksum found for %s", filename)
	}

	if fileInfo, err := os.Stat(keyPath); err == nil {
		logging.Logger().Info().
			Str("file", filename).
			Int64("size", fileInfo.Size()).
			Msg("Verifying existing key file")

		valid, err := verifyChecksum(keyPath, expectedChecksum)
		if err != nil {
			logging.Logger().Warn().
				Err(err).
				Str("file", filename).
				Msg("Failed to verify checksum, will re-download")
		} else if valid {
			logging.Logger().Info().
				Str("file", filename).
				Msg("Key file is valid, skipping download")
			return nil
		} else {
			logging.Logger().Warn().
				Str("file", filename).
				Msg("Checksum mismatch, re-downloading")
			os.Remove(keyPath)
		}
	}

	if err := os.MkdirAll(filepath.Dir(keyPath), 0755); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	url := fmt.Sprintf("%s/%s", config.BaseURL, filename)
	logging.Logger().Info().
		Str("file", filename).
		Str("url", url).
		Msg("Downloading key file")

	if err := downloadFileWithResume(url, keyPath, config); err != nil {
		return err
	}

	valid, err := verifyChecksum(keyPath, expectedChecksum)
	if err != nil {
		return fmt.Errorf("failed to verify downloaded file: %w", err)
	}
	if !valid {
		os.Remove(keyPath)
		return fmt.Errorf("downloaded file checksum mismatch")
	}

	logging.Logger().Info().
		Str("file", filename).
		Msg("Key file downloaded and verified successfully")

	return nil
}

func EnsureKeysExist(keys []string, config *DownloadConfig) error {
	if !config.AutoDownload {
		for _, key := range keys {
			if _, err := os.Stat(key); os.IsNotExist(err) {
				return fmt.Errorf("required key file not found: %s (auto-download disabled)", key)
			}
		}
		return nil
	}

	if err := downloadChecksum(config); err != nil {
		return fmt.Errorf("failed to download checksums: %w", err)
	}

	var missingKeys []string
	for _, key := range keys {
		if _, err := os.Stat(key); os.IsNotExist(err) {
			missingKeys = append(missingKeys, key)
		}
	}

	if len(missingKeys) > 0 {
		logging.Logger().Info().
			Int("missing_count", len(missingKeys)).
			Int("total_count", len(keys)).
			Msg("Found missing key files, will download")

		for i, key := range missingKeys {
			logging.Logger().Info().
				Int("current", i+1).
				Int("total", len(missingKeys)).
				Str("file", filepath.Base(key)).
				Msg("Downloading missing key")

			if err := DownloadKey(key, config); err != nil {
				return fmt.Errorf("failed to download key %s: %w", filepath.Base(key), err)
			}
		}
	}

	return nil
}
