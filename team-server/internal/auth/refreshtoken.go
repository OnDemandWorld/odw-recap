package auth

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
)

// GenerateRefreshToken returns a cryptographically random opaque token.
// Refresh tokens are bearer secrets: they are generated here, sent to the
// client once, and only ever stored server-side as SHA-256 hashes.
func GenerateRefreshToken() string {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		// crypto/rand failing is unrecoverable for authentication.
		panic("crypto/rand failed: " + err.Error())
	}
	return base64.RawURLEncoding.EncodeToString(buf)
}

// HashToken hashes a refresh token for storage/lookup. Storing only the hash
// means a leaked database does not leak usable refresh tokens.
func HashToken(token string) string {
	sum := sha256.Sum256([]byte(token))
	return hex.EncodeToString(sum[:])
}
