package main

import (
	"embed"
	"log"
	"os"
	"path/filepath"
	"strings"

	appbackend "github.com/alexfedura/vora_mind/go-app/backend/application"
	"github.com/alexfedura/vora_mind/go-app/backend/infrastructure"
	"github.com/alexfedura/vora_mind/go-app/backend/persistence"
	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	configDir, err := os.UserConfigDir()
	if err != nil {
		log.Fatal(err)
	}
	dataDir := strings.TrimSpace(os.Getenv("VORA_GO_DATA_DIR"))
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

	allowedRoots := []string{dataDir}
	if extra := strings.TrimSpace(os.Getenv("VORA_ALLOWED_FS_ROOTS")); extra != "" {
		for _, root := range strings.Split(extra, string(os.PathListSeparator)) {
			if strings.TrimSpace(root) != "" {
				allowedRoots = append(allowedRoots, root)
			}
		}
	}
	filesystem, err := infrastructure.NewFilesystemScope(allowedRoots...)
	if err != nil {
		store.DB.Close()
		log.Fatal(err)
	}

	runtime := appbackend.NewRuntime(store, filesystem, "Vora Mind", "0.26.5", "v3.1.0")
	runtimeService := NewRuntimeService(runtime, store)

	app := application.New(application.Options{
		Name:        "Vora Mind",
		Description: "AI-operable local-first research environment foundation",
		Services: []application.Service{
			application.NewService(runtimeService),
		},
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
		Mac: application.MacOptions{
			ApplicationShouldTerminateAfterLastWindowClosed: true,
		},
	})

	app.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:  "Vora Mind",
		Width:  1280,
		Height: 800,
		Mac: application.MacWindow{
			InvisibleTitleBarHeight: 50,
			Backdrop:                application.MacBackdropTranslucent,
			TitleBar:                application.MacTitleBarHiddenInset,
		},
		BackgroundColour: application.NewRGB(18, 20, 27),
		URL:              "/",
	})

	if err := app.Run(); err != nil {
		log.Fatal(err)
	}
}
