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
	Permissions []Permission `json:"permissions"`
	CreatedAt   time.Time    `json:"createdAt"`
	RevokedAt   *time.Time   `json:"revokedAt,omitempty"`
	LastUsedAt  *time.Time   `json:"lastUsedAt,omitempty"`
	Status      string       `json:"status"`
}

type CredentialManager struct {
	db         *sql.DB
	mu         sync.RWMutex
	activeHash string
}

func NewCredentialManager(db *sql.DB) *CredentialManager {
	return &CredentialManager{db: db}
}

func (m *CredentialManager) Generate(permissions []Permission) (Credential, string, error) {
	random := make([]byte, 32)
	if _, err := rand.Read(random); err != nil {
		return Credential{}, "", err
	}
	secret := hex.EncodeToString(random)
	now := time.Now().UTC()
	id := "credential_" + hex.EncodeToString(random[:8])
	hash := sha256.Sum256([]byte(secret))
	hashText := hex.EncodeToString(hash[:])
	permissionsJSON, err := json.Marshal(permissions)
	if err != nil {
		return Credential{}, "", err
	}

	m.mu.Lock()
	defer m.mu.Unlock()
	if _, err := m.db.Exec(`INSERT INTO api_credentials(id,credential_hash,encrypted_secret,permissions_json,created_at) VALUES(?,?,?,?,?)`,
		id, hashText, "", string(permissionsJSON), now.Format(time.RFC3339Nano)); err != nil {
		return Credential{}, "", err
	}
	m.activeHash = hashText
	return Credential{ID: id, Permissions: permissions, CreatedAt: now, Status: "active"}, secret, nil
}

func (m *CredentialManager) Authenticate(secret string) (Credential, bool) {
	hash := sha256.Sum256([]byte(strings.TrimSpace(secret)))
	hashText := hex.EncodeToString(hash[:])
	m.mu.RLock()
	defer m.mu.RUnlock()
	var id, permissionsJSON, created string
	var revoked, lastUsed sql.NullString
	err := m.db.QueryRow(`
		SELECT id,permissions_json,created_at,revoked_at,last_used_at
		FROM api_credentials
		WHERE credential_hash=? AND revoked_at IS NULL`, hashText).Scan(&id, &permissionsJSON, &created, &revoked, &lastUsed)
	if err != nil {
		return Credential{}, false
	}
	credential := Credential{ID: id, Status: "active"}
	_ = json.Unmarshal([]byte(permissionsJSON), &credential.Permissions)
	credential.CreatedAt, _ = time.Parse(time.RFC3339Nano, created)
	if lastUsed.Valid {
		parsed, _ := time.Parse(time.RFC3339Nano, lastUsed.String)
		credential.LastUsedAt = &parsed
	}
	_ = subtle.ConstantTimeCompare([]byte(hashText), []byte(hashText))
	now := time.Now().UTC().Format(time.RFC3339Nano)
	_, _ = m.db.Exec(`UPDATE api_credentials SET last_used_at=? WHERE credential_hash=?`, now, hashText)
	return credential, true
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
	rows, err := m.db.Query(`SELECT id,permissions_json,created_at,revoked_at,last_used_at FROM api_credentials ORDER BY created_at DESC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	result := make([]Credential, 0)
	for rows.Next() {
		var item Credential
		var permissionsJSON, created string
		var revoked, lastUsed sql.NullString
		if err := rows.Scan(&item.ID, &permissionsJSON, &created, &revoked, &lastUsed); err != nil {
			return nil, err
		}
		_ = json.Unmarshal([]byte(permissionsJSON), &item.Permissions)
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
		result = append(result, item)
	}
	return result, rows.Err()
}

func EncodeSecretForOSProtection(secret []byte) string {
	return base64.RawStdEncoding.EncodeToString(secret)
}
