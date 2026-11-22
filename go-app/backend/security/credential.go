package security

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"strings"
	"sync"
	"time"

	"database/sql"
)

type Credential struct {
	ID          string       `json:"id"`
	Name        string       `json:"name"`
	Permissions []Permission `json:"permissions"`
	ProjectIDs  []string     `json:"projectIds"`
	CreatedAt   time.Time    `json:"createdAt"`
	RevokedAt   *time.Time   `json:"revokedAt,omitempty"`
	LastUsedAt  *time.Time   `json:"lastUsedAt,omitempty"`
	ExpiresAt   *time.Time   `json:"expiresAt,omitempty"`
	Status      string       `json:"status"`
}

type CredentialManager struct {
	db         *sql.DB
	mu         sync.RWMutex
	activeHash string
	provider   SecretProvider
}

func NewCredentialManager(db *sql.DB) *CredentialManager {
	provider, err := defaultSecretProvider()
	if err != nil {
		return &CredentialManager{db: db}
	}
	return &CredentialManager{db: db, provider: provider}
}

func (m *CredentialManager) Generate(permissions []Permission) (Credential, string, error) {
	return m.GenerateNamed("API credential", permissions, nil, nil)
}

func (m *CredentialManager) GenerateNamed(name string, permissions []Permission, projectIDs []string, expiresAt *time.Time) (Credential, string, error) {
	name = strings.TrimSpace(name)
	if name == "" {
		return Credential{}, "", errors.New("credential name is required")
	}
	if m.provider == nil {
		return Credential{}, "", errors.New("OS credential protection provider is unavailable")
	}
	random := make([]byte, 32)
	if _, err := rand.Read(random); err != nil {
		return Credential{}, "", err
	}
	secret := hex.EncodeToString(random)
	now := time.Now().UTC()
	id := "credential_" + hex.EncodeToString(random[:8])
	hash := sha256.Sum256([]byte(secret))
	hashText := hex.EncodeToString(hash[:])
	protected, err := m.provider.Protect([]byte(secret))
	if err != nil {
		return Credential{}, "", err
	}
	permissionsJSON, err := json.Marshal(permissions)
	if err != nil {
		return Credential{}, "", err
	}
	projectJSON, err := json.Marshal(projectIDs)
	if err != nil {
		return Credential{}, "", err
	}
	var expiryValue interface{}
	if expiresAt != nil {
		expiryValue = expiresAt.UTC().Format(time.RFC3339Nano)
	}

	m.mu.Lock()
	defer m.mu.Unlock()
	if _, err := m.db.Exec(`INSERT INTO api_credentials(id,name,credential_hash,encrypted_secret,permissions_json,project_ids_json,expires_at,protection_provider,created_at) VALUES(?,?,?,?,?,?,?,?,?)`,
		id, name, hashText, base64.RawStdEncoding.EncodeToString(protected), string(permissionsJSON), string(projectJSON), expiryValue, m.provider.Name(), now.Format(time.RFC3339Nano)); err != nil {
		return Credential{}, "", err
	}
	m.activeHash = hashText
	return Credential{ID: id, Name: name, Permissions: permissions, ProjectIDs: projectIDs, ExpiresAt: expiresAt, CreatedAt: now, Status: "active"}, secret, nil
}

func (m *CredentialManager) Authenticate(secret string) (Credential, bool) {
	hash := sha256.Sum256([]byte(strings.TrimSpace(secret)))
	hashText := hex.EncodeToString(hash[:])
	m.mu.RLock()
	defer m.mu.RUnlock()
	var id, name, encryptedText, permissionsJSON, projectJSON, created string
	var revoked, lastUsed, expires, providerName sql.NullString
	err := m.db.QueryRow(`
		SELECT id,name,encrypted_secret,permissions_json,project_ids_json,created_at,revoked_at,last_used_at,expires_at,protection_provider
		FROM api_credentials
		WHERE credential_hash=? AND revoked_at IS NULL`,
		hashText).Scan(&id, &name, &encryptedText, &permissionsJSON, &projectJSON, &created, &revoked, &lastUsed, &expires, &providerName)
	if err != nil {
		return Credential{}, false
	}
	if expires.Valid {
		expiry, err := time.Parse(time.RFC3339Nano, expires.String)
		if err != nil || !expiry.After(time.Now().UTC()) {
			return Credential{}, false
		}
	}
	if m.provider == nil {
		return Credential{}, false
	}
	protected, err := base64.RawStdEncoding.DecodeString(encryptedText)
	if err != nil {
		return Credential{}, false
	}
	plaintext, err := m.provider.Unprotect(protected)
	if err != nil {
		return Credential{}, false
	}
	plaintextHash := sha256.Sum256(plaintext)
	if subtle.ConstantTimeCompare(plaintextHash[:], hash[:]) != 1 {
		return Credential{}, false
	}
	credential := Credential{ID: id, Name: name, Status: "active"}
	_ = json.Unmarshal([]byte(permissionsJSON), &credential.Permissions)
	_ = json.Unmarshal([]byte(projectJSON), &credential.ProjectIDs)
	credential.CreatedAt, _ = time.Parse(time.RFC3339Nano, created)
	if lastUsed.Valid {
		parsed, _ := time.Parse(time.RFC3339Nano, lastUsed.String)
		credential.LastUsedAt = &parsed
	}
	now := time.Now().UTC().Format(time.RFC3339Nano)
	_, _ = m.db.Exec(`UPDATE api_credentials SET last_used_at=? WHERE credential_hash=?`, now, hashText)
	return credential, true
}

func (m *CredentialManager) Rotate(id string) (Credential, string, error) {
	m.mu.Lock()
	var name, permissionsJSON, projectJSON, expires string
	err := m.db.QueryRow(`SELECT name,permissions_json,project_ids_json,COALESCE(expires_at,'') FROM api_credentials WHERE id=? AND revoked_at IS NULL`, id).
		Scan(&name, &permissionsJSON, &projectJSON, &expires)
	if err != nil {
		m.mu.Unlock()
		return Credential{}, "", errors.New("active credential not found")
	}
	if _, err := m.db.Exec(`UPDATE api_credentials SET revoked_at=? WHERE id=? AND revoked_at IS NULL`,
		time.Now().UTC().Format(time.RFC3339Nano), id); err != nil {
		m.mu.Unlock()
		return Credential{}, "", err
	}
	m.mu.Unlock()
	var permissions []Permission
	var projects []string
	_ = json.Unmarshal([]byte(permissionsJSON), &permissions)
	_ = json.Unmarshal([]byte(projectJSON), &projects)
	var expiry *time.Time
	if expires != "" {
		parsed, err := time.Parse(time.RFC3339Nano, expires)
		if err == nil {
			expiry = &parsed
		}
	}
	return m.GenerateNamed(name, permissions, projects, expiry)
}

func (m *CredentialManager) Revoke(id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	result, err := m.db.Exec(`UPDATE api_credentials SET revoked_at=? WHERE id=? AND revoked_at IS NULL`,
		time.Now().UTC().Format(time.RFC3339Nano), id)
	if err != nil {
		return err
	}
	count, _ := result.RowsAffected()
	if count == 0 {
		return errors.New("credential not found or already revoked")
	}
	m.activeHash = ""
	return nil
}

func (m *CredentialManager) List() ([]Credential, error) {
	rows, err := m.db.Query(`SELECT id,name,permissions_json,project_ids_json,created_at,revoked_at,last_used_at,expires_at FROM api_credentials ORDER BY created_at DESC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	result := make([]Credential, 0)
	for rows.Next() {
		var item Credential
		var permissionsJSON, projectJSON, created string
		var revoked, lastUsed, expires sql.NullString
		if err := rows.Scan(&item.ID, &item.Name, &permissionsJSON, &projectJSON, &created, &revoked, &lastUsed, &expires); err != nil {
			return nil, err
		}
		_ = json.Unmarshal([]byte(permissionsJSON), &item.Permissions)
		_ = json.Unmarshal([]byte(projectJSON), &item.ProjectIDs)
		item.CreatedAt, _ = time.Parse(time.RFC3339Nano, created)
		item.Status = "active"
		if revoked.Valid {
			parsed, _ := time.Parse(time.RFC3339Nano, revoked.String)
			item.RevokedAt = &parsed
			item.Status = "revoked"
		}
		if lastUsed.Valid {
			parsed, _ := time.Parse(time.RFC3339Nano, lastUsed.String)
			item.LastUsedAt = &parsed
		}
		if expires.Valid {
			parsed, _ := time.Parse(time.RFC3339Nano, expires.String)
			item.ExpiresAt = &parsed
		}
		result = append(result, item)
	}
	return result, rows.Err()
}

func EncodeSecretForOSProtection(secret []byte) string {
	return base64.RawStdEncoding.EncodeToString(secret)
}
