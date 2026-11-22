package security

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"
	"fmt"
	"io"
	"os"
	"runtime"
)

// SecretProvider protects credential plaintext with a platform trust boundary.
// Implementations must never return plaintext from storage or logs.
type SecretProvider interface {
	Protect(secret []byte) ([]byte, error)
	Unprotect(ciphertext []byte) ([]byte, error)
	Name() string
}

func defaultSecretProvider() (SecretProvider, error) {
	if runtime.GOOS == "windows" {
		return newWindowsDPAPIProvider()
	}
	if os.Getenv("VORA_CREDENTIAL_TEST_PROVIDER") == "1" {
		return newTestSecretProvider()
	}
	return nil, errors.New("no OS credential protection provider is available; set VORA_CREDENTIAL_TEST_PROVIDER=1 only for isolated development/tests")
}

type testSecretProvider struct {
	aead cipher.AEAD
}

func newTestSecretProvider() (SecretProvider, error) {
	key := make([]byte, 32)
	if _, err := io.ReadFull(rand.Reader, key); err != nil {
		return nil, err
	}
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	return &testSecretProvider{aead: aead}, nil
}

func (p *testSecretProvider) Name() string { return "isolated-test-only" }

func (p *testSecretProvider) Protect(secret []byte) ([]byte, error) {
	nonce := make([]byte, p.aead.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	return append(nonce, p.aead.Seal(nil, nonce, secret, nil)...), nil
}

func (p *testSecretProvider) Unprotect(ciphertext []byte) ([]byte, error) {
	if len(ciphertext) < p.aead.NonceSize() {
		return nil, fmt.Errorf("invalid ciphertext")
	}
	nonce := ciphertext[:p.aead.NonceSize()]
	return p.aead.Open(nil, nonce, ciphertext[p.aead.NonceSize():], nil)
}
