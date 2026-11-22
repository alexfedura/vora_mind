package main

import (
	"context"
	"log"
	"os"
	"path/filepath"

	appbackend "github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	mcpgateway "github.com/alexfedura/vora_mind/go-app/backend/mcp"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
)

func main() {
	configDir, err := os.UserConfigDir()
	if err != nil {
		log.Fatal(err)
	}
	dataDir := os.Getenv("VORA_GO_DATA_DIR")
	if dataDir == "" {
		dataDir = filepath.Join(configDir, "Vora Mind")
	}
	if err := os.MkdirAll(dataDir, 0o700); err != nil {
		log.Fatal(err)
	}
	store, err := persistence.Open(filepath.Join(dataDir, "vora-phase1.db"))
	if err != nil {
		log.Fatal(err)
	}
	defer store.DB.Close()
	filesystem, err := infrastructure.NewFilesystemScope(dataDir)
	if err != nil {
		log.Fatal(err)
	}
	runtime := appbackend.NewRuntime(store, filesystem, "Vora Mind", "0.26.10", "v3.1.0")
	gateway := mcpgateway.NewGateway(runtime)
	if err := gateway.Run(context.Background()); err != nil {
		log.Printf("MCP server stopped: %v", err)
		os.Exit(1)
	}
}
