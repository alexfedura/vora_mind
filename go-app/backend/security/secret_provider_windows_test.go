//go:build windows

package security

import (
	"bytes"
	"crypto/rand"
	"testing"
)

func TestWindowsDPAPIProtectUnprotectRoundTrip(t *testing.T) {
	provider, err := newWindowsDPAPIProvider()
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
	if len(protected) == 0 {
		t.Fatal("DPAPI returned empty ciphertext")
	}
	unprotected, err := provider.Unprotect(protected)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(secret, unprotected) {
		t.Fatal("DPAPI roundtrip changed the secret")
	}
}

func TestWindowsDPAPIRejectsInvalidAndCorruptedCiphertext(t *testing.T) {
	provider, err := newWindowsDPAPIProvider()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := provider.Protect(nil); err == nil {
		t.Fatal("empty DPAPI plaintext unexpectedly succeeded")
	}
	if _, err := provider.Unprotect([]byte("corrupted")); err == nil {
		t.Fatal("corrupted ciphertext unexpectedly decrypted")
	}
	ciphertext, err := provider.Protect([]byte("dpapi-ciphertext"))
	if err != nil {
		t.Fatal(err)
	}
	ciphertext[len(ciphertext)-1] ^= 0xff
	if _, err := provider.Unprotect(ciphertext); err == nil {
		t.Fatal("corrupted DPAPI ciphertext unexpectedly decrypted")
	}
}
