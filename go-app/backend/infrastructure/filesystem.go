package infrastructure

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

type FilesystemScope struct {
	Roots []string
}

func NewFilesystemScope(roots ...string) (*FilesystemScope, error) {
	normalized := make([]string, 0, len(roots))
	for _, root := range roots {
		absolute, err := filepath.Abs(root)
		if err != nil {
			return nil, err
		}
		absolute = filepath.Clean(absolute)
		if err := os.MkdirAll(absolute, 0o700); err != nil {
			return nil, err
		}
		normalized = append(normalized, absolute)
	}
	if len(normalized) == 0 {
		return nil, fmt.Errorf("at least one filesystem scope is required")
	}
	sort.Strings(normalized)
	return &FilesystemScope{Roots: normalized}, nil
}

func (s *FilesystemScope) resolve(relative string) (string, error) {
	relative = strings.TrimSpace(relative)
	if relative == "" {
		return "", fmt.Errorf("path is required")
	}
	var absoluteCandidate string
	if filepath.IsAbs(relative) {
		absoluteCandidate = filepath.Clean(relative)
	} else {
		resolved, err := filepath.Abs(filepath.Join(s.Roots[0], relative))
		if err != nil {
			return "", err
		}
		absoluteCandidate = filepath.Clean(resolved)
	}
	canonicalCandidate, err := canonicalExistingPath(absoluteCandidate)
	if err != nil {
		return "", err
	}
	for _, root := range s.Roots {
		canonicalRoot, err := filepath.EvalSymlinks(root)
		if err != nil {
			continue
		}
		if pathWithinRoot(canonicalRoot, canonicalCandidate) {
			return canonicalCandidate, nil
		}
	}
	return "", fmt.Errorf("path is outside allowed filesystem scopes")
}

func canonicalExistingPath(path string) (string, error) {
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		return filepath.Clean(resolved), nil
	}
	current := filepath.Clean(path)
	suffix := make([]string, 0)
	for {
		resolved, err := filepath.EvalSymlinks(current)
		if err == nil {
			for i := len(suffix) - 1; i >= 0; i-- {
				resolved = filepath.Join(resolved, suffix[i])
			}
			return filepath.Clean(resolved), nil
		}
		parent := filepath.Dir(current)
		if parent == current {
			return filepath.Clean(path), nil
		}
		suffix = append(suffix, filepath.Base(current))
		current = parent
	}
}

func pathWithinRoot(root, path string) bool {
	relative, err := filepath.Rel(root, path)
	if err != nil {
		return false
	}
	return relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator))
}

func (s *FilesystemScope) List(relative string) ([]map[string]interface{}, error) {
	path, err := s.resolve(relative)
	if err != nil {
		return nil, err
	}
	entries, err := os.ReadDir(path)
	if err != nil {
		return nil, err
	}
	result := make([]map[string]interface{}, 0, len(entries))
	for _, entry := range entries {
		info, err := entry.Info()
		if err != nil {
			return nil, err
		}
		result = append(result, map[string]interface{}{
			"name": entry.Name(), "isDirectory": entry.IsDir(),
			"size": info.Size(), "modifiedAt": info.ModTime().UTC().Format("2006-01-02T15:04:05Z07:00"),
		})
	}
	return result, nil
}

func (s *FilesystemScope) Stat(relative string) (map[string]interface{}, error) {
	path, err := s.resolve(relative)
	if err != nil {
		return nil, err
	}
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	return map[string]interface{}{
		"path": path, "name": info.Name(), "isDirectory": info.IsDir(),
		"size": info.Size(), "modifiedAt": info.ModTime().UTC().Format("2006-01-02T15:04:05Z07:00"),
	}, nil
}

func (s *FilesystemScope) ReadText(relative string) (map[string]interface{}, error) {
	path, err := s.resolve(relative)
	if err != nil {
		return nil, err
	}
	content, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return map[string]interface{}{"path": path, "content": string(content)}, nil
}

func (s *FilesystemScope) CreateText(relative, content string) (map[string]interface{}, error) {
	path, err := s.resolve(relative)
	if err != nil {
		return nil, err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return nil, err
	}
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		return nil, err
	}
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	return map[string]interface{}{"path": path, "size": info.Size()}, nil
}

func (s *FilesystemScope) Rename(source, destination string) (map[string]interface{}, error) {
	src, err := s.resolve(source)
	if err != nil {
		return nil, err
	}
	dst, err := s.resolve(destination)
	if err != nil {
		return nil, err
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o700); err != nil {
		return nil, err
	}
	if err := os.Rename(src, dst); err != nil {
		return nil, err
	}
	return map[string]interface{}{"source": src, "destination": dst}, nil
}

func (s *FilesystemScope) Copy(source, destination string) (map[string]interface{}, error) {
	src, err := s.resolve(source)
	if err != nil {
		return nil, err
	}
	dst, err := s.resolve(destination)
	if err != nil {
		return nil, err
	}
	info, err := os.Stat(src)
	if err != nil {
		return nil, err
	}
	if info.IsDir() {
		return nil, fmt.Errorf("directory copy is not supported")
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o700); err != nil {
		return nil, err
	}
	input, err := os.ReadFile(src)
	if err != nil {
		return nil, err
	}
	if err := os.WriteFile(dst, input, 0o600); err != nil {
		return nil, err
	}
	return map[string]interface{}{"source": src, "destination": dst, "size": int64(len(input))}, nil
}

func (s *FilesystemScope) Move(source, destination string) (map[string]interface{}, error) {
	return s.Rename(source, destination)
}

func (s *FilesystemScope) Delete(relative string) (map[string]interface{}, error) {
	path, err := s.resolve(relative)
	if err != nil {
		return nil, err
	}
	info, err := os.Lstat(path)
	if err != nil {
		return nil, err
	}
	if info.IsDir() {
		return nil, fmt.Errorf("directory deletion is not supported")
	}
	if err := os.Remove(path); err != nil {
		return nil, err
	}
	return map[string]interface{}{"path": path, "deleted": true}, nil
}
