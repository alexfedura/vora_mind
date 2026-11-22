package security

import (
	"crypto/rand"
	"testing"
)

func TestTestSecretProviderRoundTrip(t *testing.T) {
	provider, err := newTestSecretProvider()
	if err != nil {
		t.Fatal(err)
	}
	secret := make([]byte, 32)
	if _, err := rand.Read(secret); err != nil {
		t.Fatal(err)
	}
	protected, err := provider.Protect(secret)
	if err != nil {
		t.Fatal(err)
	}
	unprotected, err := provider.Unprotect(protected)
	if err != nil {
		t.Fatal(err)
	}
	if string(unprotected) != string(secret) {
		t.Fatal("secret did not round-trip")
	}
}
