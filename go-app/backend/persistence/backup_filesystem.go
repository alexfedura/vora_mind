package persistence

import (
	"os"
	"path/filepath"
)

type backupFilesystem interface {
	Abs(path string) (string, error)
	EvalSymlinks(path string) (string, error)
	Rel(basepath, targpath string) (string, error)
	SameFile(pathA, pathB string, infoA, infoB os.FileInfo) bool
	MkdirAll(path string, perm os.FileMode) error
	ReadDir(path string) ([]os.DirEntry, error)
	ReadFile(path string) ([]byte, error)
	WriteFile(path string, data []byte, perm os.FileMode) error
	Remove(path string) error
	Rename(oldpath, newpath string) error
	Stat(path string) (os.FileInfo, error)
	CreateTemp(dir, pattern string) (*os.File, error)
}

type osBackupFilesystem struct{}

func (osBackupFilesystem) Abs(path string) (string, error) {
	return filepath.Abs(path)
}

func (osBackupFilesystem) EvalSymlinks(path string) (string, error) {
	return filepath.EvalSymlinks(path)
}

func (osBackupFilesystem) Rel(basepath, targpath string) (string, error) {
	return filepath.Rel(basepath, targpath)
}

func (osBackupFilesystem) SameFile(pathA, pathB string, infoA, infoB os.FileInfo) bool {
	if stringsEqualFoldPath(pathA, pathB) {
		return true
	}
	return os.SameFile(infoA, infoB)
}

func (osBackupFilesystem) MkdirAll(path string, perm os.FileMode) error {
	return os.MkdirAll(path, perm)
}

func (osBackupFilesystem) ReadDir(path string) ([]os.DirEntry, error) {
	return os.ReadDir(path)
}

func (osBackupFilesystem) ReadFile(path string) ([]byte, error) {
	return os.ReadFile(path)
}

func (osBackupFilesystem) WriteFile(path string, data []byte, perm os.FileMode) error {
	return os.WriteFile(path, data, perm)
}

func (osBackupFilesystem) Remove(path string) error {
	return os.Remove(path)
}

func (osBackupFilesystem) Rename(oldpath, newpath string) error {
	return os.Rename(oldpath, newpath)
}

func (osBackupFilesystem) Stat(path string) (os.FileInfo, error) {
	return os.Stat(path)
}

func (osBackupFilesystem) CreateTemp(dir, pattern string) (*os.File, error) {
	return os.CreateTemp(dir, pattern)
}
