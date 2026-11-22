package persistence

import (
	"database/sql"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

type BackupManager struct {
	store *Store
	root  string
}

type BackupRecord struct {
	Path      string    `json:"path"`
	Size      int64     `json:"size"`
	CreatedAt time.Time `json:"createdAt"`
	Valid     bool      `json:"valid"`
}

func NewBackupManager(store *Store, root string) (*BackupManager, error) {
	if err := os.MkdirAll(root, 0o700); err != nil {
		return nil, err
	}
	return &BackupManager{store: store, root: root}, nil
}

func (m *BackupManager) Create() (BackupRecord, error) {
	if err := os.MkdirAll(m.root, 0o700); err != nil {
		return BackupRecord{}, err
	}
	name := fmt.Sprintf("vora-backup-%s.db", time.Now().UTC().Format("20060102T150405.000000000Z"))
	target := filepath.Join(m.root, name)
	temp, err := os.CreateTemp(m.root, ".vora-backup-*.tmp")
	if err != nil {
		return BackupRecord{}, err
	}
	tempPath := temp.Name()
	temp.Close()
	_ = os.Remove(tempPath)

	quote := "'" + strings.ReplaceAll(tempPath, "'", "''") + "'"
	_, err = m.store.DB.Exec("VACUUM INTO " + quote)
	if err != nil {
		_ = os.Remove(tempPath)
		return BackupRecord{}, err
	}
	if err := os.Rename(tempPath, target); err != nil {
		_ = os.Remove(tempPath)
		return BackupRecord{}, err
	}
	valid, err := ValidateDatabase(target)
	if err != nil {
		_ = os.Remove(target)
		return BackupRecord{}, err
	}
	info, err := os.Stat(target)
	if err != nil {
		return BackupRecord{}, err
	}
	return BackupRecord{Path: target, Size: info.Size(), CreatedAt: time.Now().UTC(), Valid: valid}, nil
}

func ValidateDatabase(path string) (bool, error) {
	db, err := sql.Open("sqlite", "file:"+filepath.ToSlash(path)+"?mode=ro")
	if err != nil {
		return false, err
	}
	defer db.Close()
	var integrity string
	if err := db.QueryRow("PRAGMA integrity_check").Scan(&integrity); err != nil {
		return false, err
	}
	if integrity != "ok" {
		return false, fmt.Errorf("backup integrity check failed: %s", integrity)
	}
	var foreignKey string
	rows, err := db.Query("PRAGMA foreign_key_check")
	if err != nil {
		return false, err
	}
	defer rows.Close()
	if rows.Next() {
		foreignKey = "foreign-key violation"
	}
	if foreignKey != "" {
		return false, errors.New(foreignKey)
	}
	return true, nil
}

func (m *BackupManager) List() ([]BackupRecord, error) {
	entries, err := os.ReadDir(m.root)
	if err != nil {
		return nil, err
	}
	records := make([]BackupRecord, 0)
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".db" {
			continue
		}
		path := filepath.Join(m.root, entry.Name())
		info, err := entry.Info()
		if err != nil {
			return nil, err
		}
		valid, err := ValidateDatabase(path)
		if err != nil {
			valid = false
		}
		records = append(records, BackupRecord{Path: path, Size: info.Size(), CreatedAt: info.ModTime().UTC(), Valid: valid})
	}
	return records, nil
}

func (m *BackupManager) PreflightRestore(path string) (bool, error) {
	absolute, err := filepath.Abs(path)
	if err != nil {
		return false, err
	}
	allowed, err := filepath.Rel(m.root, absolute)
	if err != nil || allowed == ".." || filepath.IsAbs(allowed) {
		return false, errors.New("restore source must be inside the backup directory")
	}
	return ValidateDatabase(absolute)
}
