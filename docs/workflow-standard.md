# Vora Mind — Git Workflow Standard

**Version:** 2.2.1
**Status:** MANDATORY
**Effective From:** 2026-09-28

---

## 1. The Standard Cycle

Every feature, bugfix, or chunk MUST follow this exact cycle:

```
┌──────────────────────────────────────────────────────────────┐
│  STEP 1: Branch                                              │
│    Create a branch named EXACTLY after the feature/bug       │
│    Format: <type>/<chunk-name>-<short-desc>                  │
│    Types: feat | fix | refactor | docs | chore               │
│    Example: feat/mkt-6b-1.5-atr-range                        │
├──────────────────────────────────────────────────────────────┤
│  STEP 2: Implement                                           │
│    ALL changes (code, tests, docs) happen on this branch.    │
│    Never touch main directly.                                │
├──────────────────────────────────────────────────────────────┤
│  STEP 3: Test (GATE) — WITH WATCHDOG PAUSE                   │
│    3.0 Pause watchdog (avoid cargo target-lock contention):  │
│          vora.ps1 -Pause       (idempotent)                  │
│          sleep 2s                                            │
│    3.1 Run tests:                                            │
│      - cargo check --message-format=short                    │
│      - cargo test --lib market                               │
│      - pnpm tsc --noEmit                                     │
│      - pnpm build                                            │
│      - CLI smoke test (if applicable)                        │
│    3.2 ALWAYS resume watchdog (even on failure):             │
│          vora.ps1 -Resume                                    │
│    If ANY test fails → STOP, do NOT merge.                   │
├──────────────────────────────────────────────────────────────┤
│  STEP 4: Merge to main                                       │
│    git checkout main                                         │
│    git pull origin main                                      │
│    git merge --no-ff <branch> -m "merge: <description>"      │
│    If merge conflict → STOP, report.                         │
├──────────────────────────────────────────────────────────────┤
│  STEP 5: Delete branch                                       │
│    git branch -d <branch>                                    │
├──────────────────────────────────────────────────────────────┤
│  STEP 6: Version bump (SemVer)                               │
│    PATCH: 0.62.0 → 0.62.1  (bugfix only)                    │
│    MINOR: 0.62.0 → 0.63.0  (new feature)                    │
│    MAJOR: 0.62.0 → 1.00.0  (breaking change)                │
│    Update ALL THREE files:                                   │
│      - src-tauri/Cargo.toml                                 │
│      - src-tauri/tauri.conf.json                            │
│      - package.json                                          │
├──────────────────────────────────────────────────────────────┤
│  STEP 7: Tag                                                 │
│    git tag "backup/<chunk-name>-<timestamp>"                 │
│    git tag "vX.Y.Z"                                          │
├──────────────────────────────────────────────────────────────┤
│  STEP 8: Push                                                │
│    git push origin main                                      │
│    git push origin --tags                                    │
├──────────────────────────────────────────────────────────────┤
│  STEP 9: Report                                              │
│    Print structured report (see §7).                         │
├──────────────────────────────────────────────────────────────┤
│  STEP 10: Run (SMART, mode=AUTO, compile-aware)              │
│    1. Resume watchdog if paused (safety):                    │
│         if .vora-paused exists → vora.ps1 -Resume            │
│    2. Dev alive (vora_mind + port 1420)?                     │
│         YES → skip (HMR active)                              │
│    3. cargo/rustc compiling?                                 │
│         YES → skip (don't interfere)                         │
│    4. Else → launch via vora.ps1 + wait up to 15 min         │
│    Idempotent. Safe to run every time.                       │
├──────────────────────────────────────────────────────────────┤
│  STEP 11: Final STOP                                         │
│    Print RUN result. Wait for user feedback.                 │
│    Do NOT auto-continue to next chunk.                       │
└──────────────────────────────────────────────────────────────┘
```

---

## 2. Prompt Header

Every prompt MUST start with:

```
━━━ RUN FLAG: YES (mode=AUTO) ━━━
  AUTO: skip if dev server is alive OR cargo is compiling,
        else launch detached.
  HMR handles frontend; cargo watcher handles Rust.
  No manual restart needed unless dev server crashed.
  Watchdog PAUSE/RESUME is applied automatically around STEP 3.
```

Modes:
- `YES (mode=AUTO)` → smart: skip if alive, launch if not (DEFAULT)
- `YES (mode=FORCE)` → always kill + relaunch
- `NO` → print command for user, skip execution

Default: `YES (mode=AUTO)`.

---

## 3. Branch Naming Rules

| Type | When | Example |
|---|---|---|
| `feat/` | New feature | `feat/mkt-6b-1.5-atr-range` |
| `fix/` | Bug fix | `fix/scale-lab-layout` |
| `refactor/` | Code restructure | `refactor/market-module` |
| `docs/` | Documentation only | `docs/box-standard` |
| `chore/` | Build, deps, config | `chore/cargo-target-dir` |

Rules:
- Lowercase only
- Hyphens (not underscores)
- Max 5 words in description
- No spaces, no special chars

---

## 4. Version Bump Rules (SemVer for 0.x)

For versions < 1.0.0:

| Change Type | Bump | Example |
|---|---|---|
| Bug fix only | PATCH | 0.62.0 → 0.62.1 |
| New feature, backward-compatible | MINOR | 0.62.0 → 0.63.0 |
| Breaking change | MINOR (special) | 0.62.0 → 0.63.0 + CHANGELOG warning |

For versions >= 1.0.0:

| Change Type | Bump | Example |
|---|---|---|
| Bug fix only | PATCH | 1.0.0 → 1.0.1 |
| New feature, backward-compatible | MINOR | 1.0.0 → 1.1.0 |
| Breaking change | MAJOR | 1.0.0 → 2.0.0 |

---

## 5. Tag Structure

Two tags per release:

```
backup/<chunk-name>-<timestamp>    ← rollback safety
vX.Y.Z                             ← release
```

Example:
```
backup/mkt-6b-1.5-atr-range-20260927-183045
v0.63.0
```

Rules:
- `<chunk-name>` = same as branch name (without `feat/` or `fix/`)
- `<timestamp>` = `yyyyMMdd-HHmmss` (from `Get-Date -Format`)

---

## 6. Test Requirements (Gate)

Before merge, ALL of these MUST pass:

```powershell
# ─── STEP 3.0: Pause watchdog (idempotent) ───
cd D:\workspace\vora_mind
.\vora.ps1 -Pause
Start-Sleep -Seconds 2

# ─── STEP 3.1: Run tests ───
# Rust
cd D:\workspace\vora_mind\src-tauri
$env:CARGO_TARGET_DIR = "D:\cargo-shared-target"
cargo check --message-format=short
cargo test --lib market

# Frontend
cd D:\workspace\vora_mind
pnpm tsc --noEmit
pnpm build

# Smoke test (if chunk touches CLI or IPC)
<chunk-specific CLI commands>

# ─── STEP 3.2: ALWAYS resume watchdog (even on failure) ───
cd D:\workspace\vora_mind
.\vora.ps1 -Resume
```

If ANY fails → branch stays unmerged. Fix, re-test, then merge.

**Why Pause/Resume?** See §13 (Cargo Target Lock Contention).

---

## 7. Report Template

After STEP 9, Kiro/Cline MUST print:

```
=== BRANCH ===
<branch name> (created → merged → deleted)

=== COMMITS ===
<git log --oneline -5>

=== FILES ===
- <path> (NEW, N lines)
- <path> (MODIFIED, +N / -N)

=== TAGS ===
<git tag | Select-String for this chunk>

=== TESTS ===
cargo check: PASS/FAIL
cargo test --lib market: X passed / Y failed
pnpm tsc: PASS/FAIL (exit code)
pnpm build: PASS/FAIL

=== VERSION ===
old → new

=== PUSH ===
main: <hash> → <hash>
tags: pushed

=== STATUS ===
<git status --short>

=== NEXT ===
WAITING FOR USER TEST.
```

---

## 8. Run Step (STEP 10) — SMART MODE

### Purpose
After a successful release, ensure the app is running for visual test.
MUST be **idempotent** (safe to run every time) and **compile-aware**
(cold rebuilds can take 10–15 min).

### Behavior (mode = AUTO, default)

```
1. Resume watchdog if paused:
     if .vora-paused exists → vora.ps1 -Resume
   (Safety: if STEP 3 forgot to resume)

2. Check if dev server is FULLY ALIVE:
     - vora_mind process exists?
     - port 1420 is listening?
   ↓
   YES → skip (HMR active, no restart)

3. Check if cargo/rustc is COMPILING:
     - cargo or rustc process exists?
   ↓
   YES → skip (build in progress, don't interfere)

4. Neither alive nor compiling:
   NO  → launch via vora.ps1 (handles cleanup, PID, boot-wait)
```

### Rules
- **NEVER** kill a healthy dev server.
- **NEVER** launch if `cargo` or `rustc` is running.
- **ALWAYS** resume watchdog before checking (STEP 10.1).
- **ALWAYS** skip if `vora_mind` + port 1420 are both alive.
- **ALWAYS** delegate launch to `vora.ps1` (unified, idempotent).
- **ALWAYS** set `CARGO_TARGET_DIR = D:\cargo-shared-target`.
- **ALWAYS** allow up to **900 seconds** (15 min) for cold rebuild.

### PowerShell Template

```powershell
$TargetDir   = "D:\cargo-shared-target"
$ProjectRoot = "D:\workspace\vora_mind"
$PauseFlag   = "$ProjectRoot\.vora-paused"
$VoraScript  = "$ProjectRoot\vora.ps1"

# ─── STEP 10.1: Resume watchdog if paused (safety) ───
if (Test-Path $PauseFlag) {
    Write-Host "Resuming watchdog (found .vora-paused)" -ForegroundColor Cyan
    & $VoraScript -Resume
    Start-Sleep -Seconds 2
}

# ─── STEP 10.2: Fast paths ───
$devAlive = $false
$vm = Get-Process -Name "vora_mind" -EA SilentlyContinue
if ($vm) {
    $port = Get-NetTCPConnection -LocalPort 1420 -State Listen -EA SilentlyContinue
    if ($port) { $devAlive = $true }
}
$compiling = [bool](Get-Process -Name "cargo","rustc" -EA SilentlyContinue)

# ─── STEP 10.3: Route ───
if ($devAlive) {
    Write-Host "RUN: ALREADY RUNNING — HMR active" -ForegroundColor Green
} elseif ($compiling) {
    Write-Host "RUN: COMPILING — build in progress, skipping" -ForegroundColor Yellow
} else {
    # Delegate to vora.ps1 (does cleanup, PID tracking, boot-wait up to 15 min)
    & $VoraScript
}
```

### Update Matrix

| Change Type | What Happens | User Action |
|---|---|---|
| Frontend (.tsx/.ts) | Vite HMR reloads in ~500ms | none |
| Rust (.rs/.toml) | Cargo watcher rebuilds in ~30s | none |
| Cold rebuild (after cleanup) | ~10-15 min | wait |
| Dev server crashed | Port 1420 free → auto relaunch | none |
| Nothing changed | Detects alive server → skip | none |

### Final Report Block (STEP 11)

```
=== RUN ===
STATUS: ALREADY_RUNNING / COMPILING / LAUNCHED / TIMEOUT
HMR: ACTIVE (if ALREADY_RUNNING)
```

---

## 9. Companion Script — `vora.ps1`

Location: `D:\workspace\vora_mind\vora.ps1`

Unified launcher + watchdog with **six modes**:

```powershell
# Launch once, exit
.\vora.ps1

# Continuous monitor + auto-restart (infinite loop)
.\vora.ps1 -Watch

# Force restart
.\vora.ps1 -Force

# Cleanup only, no launch
.\vora.ps1 -CleanupOnly

# Pause watchdog (kill dev + stop monitoring)
.\vora.ps1 -Pause

# Resume watchdog (restart monitoring)
.\vora.ps1 -Resume

# Custom check interval
.\vora.ps1 -Watch -CheckEverySec 10
```

### Key Features
- **Compile-aware**: checks `cargo`/`rustc` before deciding to restart
- **Pause/Resume**: prevents cargo target-lock contention with Cline
- **Fast path**: if dev is alive + port 1420 → skip in <1s
- **Staged cleanup**: `.pdb` → incremental → deps → nuke (only if needed)
- **Disk-full detection**: parses build log for `os error 112`
- **15-min boot timeout**: handles cold rebuilds gracefully
- **Clean UTF-8 logs**: no ANSI, no mojibake, no progress bars
- **PID tracking**: kills dev pwsh tree on retry (no zombies)
- **Colored event log**: visual hierarchy (OK/WARN/ERR/STEP/DIM)
- **Logging**: writes to `.vora.log` and `.vora-build.log`

### Cargo Config

`D:\workspace\vora_mind\.cargo\config.toml` contains:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=/DEBUG:NONE"]
```

This prevents `LNK1140` ("program database limit exceeded") on
projects with heavy dependencies (Polars + Arrow + Parquet).
projects with native dependencies that need tailored linker flags.

### State Files

| File | Purpose |
|---|---|
| `.vora-paused` | Marker: watchdog is paused |
| `.vora-dev.pid` | PID of tracked dev pwsh |
| `.vora.log` | Event log |
| `.vora-build.log` | Cargo/tauri output (clean) |

---

## 10. Rules for Kiro/Cline

1.  **NEVER** work directly on main.
2.  **ALWAYS** create a branch first.
3.  **ALWAYS** pause watchdog before running tests (STEP 3.0):
    - `.\vora.ps1 -Pause` (idempotent — safe even if already paused)
    - Sleep 2s to release cargo lock
4.  **ALWAYS** resume watchdog after tests (STEP 3.2):
    - `.\vora.ps1 -Resume` — even on test failure
5.  **ALWAYS** run all tests before merge.
6.  **NEVER** merge if tests fail.
7.  **ALWAYS** bump version per SemVer rules.
8.  **ALWAYS** tag both `backup/...` and `vX.Y.Z`.
9.  **ALWAYS** push main + tags.
10. **ALWAYS** print the Report (STEP 9).
11. **ALWAYS** run STEP 10 with mode=AUTO (compile-aware, idempotent).
    - Resume watchdog first (safety).
    - Skip if `vora_mind` + port 1420 are alive.
    - Skip if `cargo` or `rustc` is running.
    - Otherwise delegate launch to `vora.ps1`.
12. **NEVER** auto-continue to the next chunk (STEP 11 STOP).

---

## 11. Cargo / NPM Output Rules

- Redirect all noisy output to a log file.
- Read only the last 10–20 lines.
- Delete the log after reading.
- No raw cargo/npm output in the report.
- On cargo dispatch failure → retry once with `--message-format=short`.

---

## 12. Environment Constants

Single source of truth for paths:

```
ProjectRoot  = D:\workspace\vora_mind
TargetDir    = D:\cargo-shared-target
CargoConfig  = D:\workspace\vora_mind\.cargo\config.toml
Launcher     = D:\workspace\vora_mind\vora.ps1
PauseFlag    = D:\workspace\vora_mind\.vora-paused
PidFile      = D:\workspace\vora_mind\.vora-dev.pid
BuildLog     = D:\workspace\vora_mind\.vora-build.log
EventLog     = D:\workspace\vora_mind\.vora.log
DevPort      = 1420
ViteVersion  = 7.x
TauriVersion = 2.x
```

**NEVER** use `C:\vora_build` — that was a legacy path.
**ALWAYS** use `D:\cargo-shared-target`.

---

## 13. Cargo Target Lock Contention

### The Problem

Both the **dev server** (watchdog-managed `pnpm tauri dev`) and
**Cline's test run** (`cargo test --lib market`) use the same
`CARGO_TARGET_DIR = D:\cargo-shared-target`.

Cargo takes an **exclusive OS-level lock** on the target directory
during any compilation. If both try to compile simultaneously:

```
Terminal A: cargo run       →  LOCK HELD
Terminal C: cargo test      →  Blocking waiting for file lock...
                            →  blocks indefinitely (or times out)
```

This can also happen when Cline edits a `.rs` file:
- Cargo watcher in dev server detects the change → triggers rebuild → lock
- Cline immediately runs `cargo test` → blocked

### The Solution: Pause/Resume

Before Cline runs any `cargo` command:

```powershell
cd D:\workspace\vora_mind
.\vora.ps1 -Pause
```

**What `-Pause` does:**
1. Kills tracked dev pwsh (and its whole process tree: cargo, rustc, vora_mind)
2. Kills anything holding port 1420
3. Writes `.vora-paused` marker file
4. If watchdog is running: it enters "paused" state and stops restarting dev

**Idempotent:** calling `-Pause` when already paused is safe (re-kills dev, re-writes flag).

After Cline's tests finish:

```powershell
.\vora.ps1 -Resume
```

**What `-Resume` does:**
1. Removes `.vora-paused` marker
2. If watchdog running: exits paused state and restarts dev on next cycle
3. If not running: does nothing (safe)

### Safety Net

If Cline crashes and forgets to resume:
- The user can run `Remove-Item .vora-paused` manually
- The watchdog will auto-resume within `CheckEverySec` seconds
- STEP 10 also calls `-Resume` at the start (defense in depth)

### What's Not Affected

- **Frontend tests** (`pnpm tsc`, `pnpm build`) — don't touch cargo target
- **Vite HMR** — runs on a separate port (1420), unaffected
- **Cline editing `.tsx` files** — HMR reloads; no pause needed

Only **Rust compilation** (`cargo *`) requires pause.

---

## 14. Change Log

| Version | Date | Changes |
|---|---|---|
| 1.0.0 | 2026-09-27 | Initial workflow (9 steps) |
| 2.0.0 | 2026-09-28 | Added STEP 10 (Run) + STEP 11 (Final STOP) |
| 2.1.0 | 2026-09-28 | Compile-aware run, fixed paths (`D:\cargo-shared-target`) |
| 2.2.0 | 2026-09-28 | Pause/Resume in STEP 3, resume-safety in STEP 10, §13 cargo lock |
| 2.2.1 | 2026-09-28 | Fix: §9 says "six modes"; §8 STEP 10 delegates to `vora.ps1`; §6 STEP 3.0 simplified (idempotent) |

---

**End of Workflow Standard v2.2.1**
