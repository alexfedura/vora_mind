# Vora Mind MCP Operational Guide

The MCP adapter uses `github.com/modelcontextprotocol/go-sdk` v1.6.1 and the
official 2025-11-25-compatible API. It communicates over stdio and never writes
protocol traffic to Wails stdout.

Build:

```powershell
cd go-app
go build -o ..\vora-mcp.exe .\cmd\vora-mcp
```

Configure an MCP client to launch:

```text
D:\workspace\vora_mind\vora-mcp.exe
```

Set `VORA_GO_DATA_DIR` only when an isolated application database directory is
desired. The MCP identity has explicit project/database/read-only filesystem
permissions. Tool discovery and invocation execute through the shared capability
runtime.
