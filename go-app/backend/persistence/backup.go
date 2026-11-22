package persistence

import (
	"database/sql"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

type BackupManager struct {
	store *Store
	root  string
	fs    backupFilesystem
}

type BackupRecord struct {
	Path      string    `json:"path"`
	Size      int64     `json:"size"`
	CreatedAt time.Time `json:"createdAt"`
	Valid     bool      `json:"valid"`
}

func NewBackupManager(store *Store, root string) (*BackupManager, error) {
	return newBackupManager(store, root, osBackupFilesystem{})
}

func newBackupManager(store *Store, root string, fs backupFilesystem) (*BackupManager, error) {
	if fs == nil {
		return nil, errors.New("backup filesystem is required")
	}
	if strings.TrimSpace(root) == "" {
		return nil, errors.New("backup root is required")
	}
	if err := fs.MkdirAll(root, 0o700); err != nil {
		return nil, err
	}
	return &BackupManager{store: store, root: root, fs: fs}, nil
}

func (m *BackupManager) Create() (BackupRecord, error) {
	if err := m.fs.MkdirAll(m.root, 0o700); err != nil {
		return BackupRecord{}, err
	}
	name := fmt.Sprintf("vora-backup-%s.db", time.Now().UTC().Format("20060102T150405.000000000Z"))
	target := filepath.Join(m.root, name)
	temp, err := m.fs.CreateTemp(m.root, ".vora-backup-*.tmp")
	if err != nil {
		return BackupRecord{}, err
	}
	tempPath := temp.Name()
	temp.Close()
	_ = m.fs.Remove(tempPath)

	quote := "'" + strings.ReplaceAll(tempPath, "'", "''") + "'"
	_, err = m.store.DB.Exec("VACUUM INTO " + quote)
	if err != nil {
		_ = m.fs.Remove(tempPath)
		return BackupRecord{}, err
	}
	if err := m.fs.Rename(tempPath, target); err != nil {
		_ = m.fs.Remove(tempPath)
		return BackupRecord{}, err
	}
	valid, err := ValidateDatabase(target)
	if err != nil {
		_ = m.fs.Remove(target)
		return BackupRecord{}, err
	}
	info, err := m.fs.Stat(target)
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
	requiredTables := []string{"schema_migrations", "projects", "audit_events", "execution_history", "workflows", "workflow_steps"}
	for _, table := range requiredTables {
		var count int
		if err := db.QueryRow("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?", table).Scan(&count); err != nil {
			return false, err
		}
		if count != 1 {
			return false, fmt.Errorf("incompatible backup: missing required table %s", table)
		}
	}
	var migrationCount int
	if err := db.QueryRow("SELECT COUNT(*) FROM schema_migrations").Scan(&migrationCount); err != nil {
		return false, fmt.Errorf("incompatible backup: migration metadata unavailable: %w", err)
	}
	if migrationCount < 2 {
		return false, errors.New("incompatible backup: migration version is too old")
	}
	if err := db.QueryRow("SELECT COUNT(*) FROM application_identity WHERE key='database_owner'").Scan(&migrationCount); err != nil {
		return false, fmt.Errorf("could not verify backup owner: %w", err)
	}
	if migrationCount == 1 {
		var owner string
		if err := db.QueryRow("SELECT value FROM application_identity WHERE key='database_owner'").Scan(&owner); err != nil {
			return false, fmt.Errorf("could not verify backup owner: %w", err)
		}
		if owner != "alexfedura/vora_mind" {
			return false, errors.New("backup belongs to a different application database")
		}
	}
	return true, nil
}

func (m *BackupManager) List() ([]BackupRecord, error) {
	entries, err := m.fs.ReadDir(m.root)
	if err != nil {
		return nil, err
	}
	records := make([]BackupRecord, 0)
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".db" {
			continue
		}
		path := filepath.Join(m.root, entry.Name())
		info, err := m.fs.Stat(path)
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

func (m *BackupManager) resolveRestorePaths(path string) (string, string, error) {
	if m.store == nil {
		return "", "", errors.New("backup store is required")
	}
	activeInput := strings.TrimSpace(m.store.Database)
	if activeInput == "" {
		return "", "", errors.New("active database path is required")
	}
	sourceInput := strings.TrimSpace(path)
	if sourceInput == "" {
		return "", "", errors.New("backup source path is required")
	}

	activeAbsolute, err := m.fs.Abs(activeInput)
	if err != nil {
		return "", "", fmt.Errorf("could not resolve active database path: %w", err)
	}
	canonicalActive, err := canonicalExistingPath(m.fs, activeAbsolute)
	if err != nil {
		return "", "", fmt.Errorf("could not canonicalize active database path: %w", err)
	}
	canonicalSource, err := canonicalExistingPath(m.fs, sourceInput)
	if err != nil {
		return "", "", fmt.Errorf("could not canonicalize backup source: %w", err)
	}

	activeInfo, err := m.fs.Stat(canonicalActive)
	if err != nil {
		return "", "", fmt.Errorf("could not inspect active database: %w", err)
	}
	sourceInfo, err := m.fs.Stat(canonicalSource)
	if err != nil {
		return "", "", fmt.Errorf("could not inspect backup source: %w", err)
	}
	if sourceInfo.IsDir() {
		return "", "", errors.New("backup source must be a file")
	}
	if sameExistingFile(m.fs, canonicalActive, canonicalSource, activeInfo, sourceInfo) {
		return "", "", errors.New("backup source and active database must be different files")
	}

	canonicalRoot, err := canonicalExistingPath(m.fs, m.root)
	if err != nil {
		return "", "", fmt.Errorf("could not canonicalize backup root: %w", err)
	}
	rootInfo, err := m.fs.Stat(canonicalRoot)
	if err != nil {
		return "", "", fmt.Errorf("could not inspect backup root: %w", err)
	}
	if !rootInfo.IsDir() {
		return "", "", errors.New("backup root must be a directory")
	}
	inside, err := pathWithinRoot(m.fs, canonicalRoot, canonicalSource)
	if err != nil {
		return "", "", fmt.Errorf("could not verify backup source scope: %w", err)
	}
	if !inside {
		return "", "", errors.New("restore source must be inside the backup directory")
	}

	return canonicalActive, canonicalSource, nil
}

func canonicalExistingPath(fs backupFilesystem, path string) (string, error) {
	input := strings.TrimSpace(path)
	if input == "" {
		return "", errors.New("path is required")
	}
	absolute, err := fs.Abs(input)
	if err != nil {
		return "", err
	}
	resolved, err := fs.EvalSymlinks(absolute)
	if err != nil {
		return "", err
	}
	if strings.TrimSpace(resolved) == "" {
		return "", errors.New("canonical path is empty")
	}
	cleaned := filepath.Clean(resolved)
	if _, err := fs.Stat(cleaned); err != nil {
		return "", err
	}
	return cleaned, nil
}

func sameExistingFile(fs backupFilesystem, pathA, pathB string, infoA, infoB os.FileInfo) bool {
	return fs.SameFile(pathA, pathB, infoA, infoB)
}

func stringsEqualFoldPath(pathA, pathB string) bool {
	cleanA := filepath.Clean(pathA)
	cleanB := filepath.Clean(pathB)
	if runtime.GOOS == "windows" {
		return strings.EqualFold(cleanA, cleanB)
	}
	return cleanA == cleanB
}

func pathWithinRoot(fs backupFilesystem, root, target string) (bool, error) {
	relative, err := fs.Rel(root, target)
	if err != nil {
		return false, err
	}
	if relative == "." {
		return false, nil
	}
	if filepath.IsAbs(relative) {
		return false, nil
	}
	if relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return false, nil
	}
	return true, nil
}
func (m *BackupManager) PreflightRestore(path string) (bool, error) {
	_, absoluteSource, err := m.resolveRestorePaths(path)
	if err != nil {
		return false, err
	}
	return ValidateDatabase(absoluteSource)
}

func (m *BackupManager) Restore(path string) (bool, error) {
	valid, err := m.PreflightRestore(path)
	if err != nil {
		return false, err
	}
	if !valid {
		return false, errors.New("backup failed validation")
	}
	absoluteActive, absoluteSource, err := m.resolveRestorePaths(path)
	if err != nil {
		return false, err
	}
	activePath := absoluteActive
	m.store.Database = activePath

	verifiedActive, verifiedSource, err := m.resolveRestorePaths(path)
	if err != nil {
		return false, fmt.Errorf("restore paths changed before quiescing: %w", err)
	}
	if verifiedActive != activePath || verifiedSource != absoluteSource {
		return false, errors.New("restore paths changed before quiescing")
	}
	if err := m.store.DB.Close(); err != nil {
		return false, fmt.Errorf("could not quiesce database: %w", err)
	}
	verifiedActive, verifiedSource, err = m.resolveRestorePaths(path)
	if err != nil {
		_ = reopenStore(m.store)
		return false, fmt.Errorf("restore paths changed after quiescing: %w", err)
	}
	if verifiedActive != activePath || verifiedSource != absoluteSource {
		_ = reopenStore(m.store)
		return false, errors.New("restore paths changed after quiescing")
	}
	recoveryPath := activePath + ".pre-restore-" + time.Now().UTC().Format("20060102T150405.000000000Z")
	if err := m.copyFile(activePath, recoveryPath); err != nil {
		_ = reopenStore(m.store)
		return false, err
	}
	stagedPath := activePath + ".restore-staging"
	if err := m.copyFile(absoluteSource, stagedPath); err != nil {
		_ = reopenStore(m.store)
		return false, err
	}
	oldPath := activePath + ".replace-old"
	_ = m.fs.Remove(oldPath)
	if err := m.fs.Rename(activePath, oldPath); err != nil {
		_ = reopenStore(m.store)
		return false, err
	}
	if err := m.fs.Rename(stagedPath, activePath); err != nil {
		_ = m.fs.Rename(oldPath, activePath)
		_ = reopenStore(m.store)
		return false, err
	}
	m.cleanupSidecars(activePath)
	if err := reopenStore(m.store); err != nil {
		_ = m.copyFile(recoveryPath, activePath)
		_ = reopenStore(m.store)
		return false, fmt.Errorf("restored database could not reopen: %w", err)
	}
	if valid, err := ValidateDatabase(activePath); err != nil || !valid {
		_ = m.copyFile(recoveryPath, activePath)
		_ = reopenStore(m.store)
		return false, fmt.Errorf("restored database failed post-replacement validation: %v", err)
	}
	_ = m.fs.Remove(oldPath)
	return true, nil
}

func (m *BackupManager) cleanupSidecars(databasePath string) {
	_ = m.fs.Remove(databasePath + "-shm")
	_ = m.fs.Remove(databasePath + "-wal")
}

func (m *BackupManager) copyFile(source, destination string) error {
	input, err := m.fs.ReadFile(source)
	if err != nil {
		return err
	}
	return m.fs.WriteFile(destination, input, 0o600)
}

func reopenStore(store *Store) error {
	reopened, err := Open(store.Database)
	if err != nil {
		return err
	}
	store.DB = reopened.DB
	store.Database = reopened.Database
	store.notifyReopened(store.DB)
	return nil
}
