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
	for _, root := range s.Roots {
		if pathWithinRoot(root, absoluteCandidate) {
			return absoluteCandidate, nil
		}
	}
	return "", fmt.Errorf("path is outside allowed filesystem scopes")
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
