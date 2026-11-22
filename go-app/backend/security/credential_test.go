package security

import (
	"path/filepath"
	"testing"
	"time"

	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
)

func testCredentialManager(t *testing.T) (*CredentialManager, func()) {
	t.Helper()
	t.Setenv("VORA_CREDENTIAL_TEST_PROVIDER", "1")
	store, err := persistence.Open(filepath.Join(t.TempDir(), "credentials.db"))
	if err != nil {
		t.Fatal(err)
	}
	return NewCredentialManager(store.DB), func() { store.DB.Close() }
}

func TestCredentialGenerateAuthenticateRevokeAndRotate(t *testing.T) {
	manager, cleanup := testCredentialManager(t)
	defer cleanup()
	credential, secret, err := manager.GenerateNamed("research", []Permission{ProjectRead}, []string{"project_1"}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if credential.Status != "active" || credential.Name != "research" {
		t.Fatalf("unexpected credential: %#v", credential)
	}
	authenticated, ok := manager.Authenticate(secret)
	if !ok || authenticated.ID != credential.ID {
		t.Fatal("credential should authenticate")
	}
	rotated, rotatedSecret, err := manager.Rotate(credential.ID)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := manager.Authenticate(secret); ok {
		t.Fatal("old credential should fail after rotation")
	}
	if authenticated, ok := manager.Authenticate(rotatedSecret); !ok || authenticated.ID != rotated.ID {
		t.Fatal("rotated credential should authenticate")
	}
	if err := manager.Revoke(rotated.ID); err != nil {
		t.Fatal(err)
	}
	if _, ok := manager.Authenticate(rotatedSecret); ok {
		t.Fatal("revoked credential should fail")
	}
}

func TestCredentialExpirationAndInvalidCiphertext(t *testing.T) {
	manager, cleanup := testCredentialManager(t)
	defer cleanup()
	expired := time.Now().UTC().Add(-time.Hour)
	credential, secret, err := manager.GenerateNamed("expired", []Permission{ProjectRead}, nil, &expired)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := manager.Authenticate(secret); ok {
		t.Fatal("expired credential should fail")
	}
	if _, err := manager.db.Exec(`UPDATE api_credentials SET encrypted_secret=? WHERE id=?`,
		"not-valid-base64", credential.ID); err != nil {
		t.Fatal(err)
	}
	if _, ok := manager.Authenticate(secret); ok {
		t.Fatal("invalid ciphertext should fail closed")
	}
}

func TestMissingProtectionProviderFailsClosed(t *testing.T) {
	store, err := persistence.Open(filepath.Join(t.TempDir(), "missing-provider.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer store.DB.Close()
	manager := &CredentialManager{db: store.DB, provider: nil}
	if _, _, err := manager.Generate([]Permission{ProjectRead}); err == nil {
		t.Fatal("missing provider should prevent credential creation")
	}
}
