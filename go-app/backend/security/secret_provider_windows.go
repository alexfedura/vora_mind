//go:build windows

package security

import (
	"fmt"
	"syscall"
	"unsafe"
)

type dpapiBlob struct {
	CbData uint32
	PbData *byte
}

type windowsDPAPIProvider struct{}

func newWindowsDPAPIProvider() (SecretProvider, error) {
	return &windowsDPAPIProvider{}, nil
}

func (p *windowsDPAPIProvider) Name() string { return "windows-dpapi" }

func (p *windowsDPAPIProvider) Protect(secret []byte) ([]byte, error) {
	crypt32 := syscall.NewLazyDLL("Crypt32.dll")
	protect := crypt32.NewProc("CryptProtectData")
	input := dpapiBlob{CbData: uint32(len(secret))}
	if len(secret) > 0 {
		input.PbData = &secret[0]
	}
	var output dpapiBlob
	r1, _, callErr := protect.Call(
		uintptr(unsafe.Pointer(&input)),
		0,
		0,
		0,
		0,
		uintptr(4),
		uintptr(unsafe.Pointer(&output)),
	)
	if r1 == 0 {
		if callErr != nil {
			return nil, fmt.Errorf("DPAPI protection failed: %w", callErr)
		}
		return nil, fmt.Errorf("DPAPI protection failed")
	}
	defer syscall.NewLazyDLL("Kernel32.dll").NewProc("LocalFree").Call(uintptr(unsafe.Pointer(output.PbData)))
	result := make([]byte, output.CbData)
	for i := range result {
		result[i] = *(*byte)(unsafe.Pointer(uintptr(unsafe.Pointer(output.PbData)) + uintptr(i)))
	}
	return result, nil
}

func (p *windowsDPAPIProvider) Unprotect(ciphertext []byte) ([]byte, error) {
	crypt32 := syscall.NewLazyDLL("Crypt32.dll")
	unprotect := crypt32.NewProc("CryptUnprotectData")
	input := dpapiBlob{CbData: uint32(len(ciphertext))}
	if len(ciphertext) > 0 {
		input.PbData = &ciphertext[0]
	}
	var output dpapiBlob
	r1, _, callErr := unprotect.Call(
		uintptr(unsafe.Pointer(&input)),
		0,
		0,
		0,
		0,
		0,
		uintptr(unsafe.Pointer(&output)),
	)
	if r1 == 0 {
		if callErr != nil {
			return nil, fmt.Errorf("DPAPI decryption failed: %w", callErr)
		}
		return nil, fmt.Errorf("DPAPI decryption failed")
	}
	defer syscall.NewLazyDLL("Kernel32.dll").NewProc("LocalFree").Call(uintptr(unsafe.Pointer(output.PbData)))
	result := make([]byte, output.CbData)
	for i := range result {
		result[i] = *(*byte)(unsafe.Pointer(uintptr(unsafe.Pointer(output.PbData)) + uintptr(i)))
	}
	return result, nil
}
