package server

import (
	"crypto/subtle"
	"light/light-prover/logging"
	"net/http"
	"os"
	"strings"
)

type authMiddleware struct {
	next   http.Handler
	apiKey string
}

func NewAPIKeyMiddleware(apiKey string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return &authMiddleware{
			next:   next,
			apiKey: apiKey,
		}
	}
}

func (m *authMiddleware) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if !m.isAuthenticated(r) {
		logging.Logger().Warn().
			Str("remote_addr", r.RemoteAddr).
			Str("path", r.URL.Path).
			Str("method", r.Method).
			Msg("Unauthorized API request - missing or invalid API key")
		
		unauthorizedError := &Error{
			StatusCode: http.StatusUnauthorized,
			Code:       "unauthorized",
			Message:    "Invalid or missing API key. Please provide a valid API key in the Authorization header as 'Bearer <api-key>' or in the X-API-Key header.",
		}
		unauthorizedError.send(w)
		return
	}

	m.next.ServeHTTP(w, r)
}

func (m *authMiddleware) isAuthenticated(r *http.Request) bool {
	if m.apiKey == "" {
		return true
	}

	providedKey := m.extractAPIKey(r)
	if providedKey == "" {
		return false
	}

	return subtle.ConstantTimeCompare([]byte(m.apiKey), []byte(providedKey)) == 1
}

func (m *authMiddleware) extractAPIKey(r *http.Request) string {
	if apiKey := r.Header.Get("X-API-Key"); apiKey != "" {
		return apiKey
	}

	if authHeader := r.Header.Get("Authorization"); authHeader != "" {
		if strings.HasPrefix(authHeader, "Bearer ") {
			return strings.TrimPrefix(authHeader, "Bearer ")
		}
	}

	return ""
}

func getAPIKeyFromEnv() string {
	return os.Getenv("PROVER_API_KEY")
}

func requiresAuthentication(path string) bool {
	publicPaths := []string{
		"/health",
	}

	for _, publicPath := range publicPaths {
		if path == publicPath {
			return false
		}
	}

	return true
}

func conditionalAuthMiddleware(apiKey string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if requiresAuthentication(r.URL.Path) {
				authHandler := NewAPIKeyMiddleware(apiKey)(next)
				authHandler.ServeHTTP(w, r)
			} else {
				next.ServeHTTP(w, r)
			}
		})
	}
}