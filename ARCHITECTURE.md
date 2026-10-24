```markdown
# Vora System Architecture & Performance Specification

Vora is a deterministic, high-performance Visual Knowledge & Code Intelligence OS built with **Tauri 2.0**, **Rust**, **React Flow**, and **Tree-sitter AST Engines**.

---

## 1. Dual-Core Architecture Overview


```

+-----------------------------------------------------------------------------+
|                                 VORA CLIENT                                 |
|                                                                             |
|   +------------------------------------+   +----------------------------+   |
|   |         React Flow Canvas          |   |      Forge AI Agent        |   |
|   |   (Visual Cards, AST Nodes, Heat)  |   |   (ReAct Loop, FTP Tools)  |   |
|   +-----------------+------------------+   +--------------+-------------+   |
|                     |                                     |                 |
|                     +------------------+------------------+                 |
|                                        | Tauri IPC Invokes / Events         |
+----------------------------------------|------------------------------------+
v
+-----------------------------------------------------------------------------+
|                              TAURI CORE (Rust)                              |
|                                                                             |
|   +------------------------------------+   +----------------------------+   |
|   |            MIND ENGINE             |   |        FORGE ENGINE        |   |
|   |  - SQLite Checkpoint/Delta SSOT    |   |  - Code Intelligence Index |   |
|   |  - Backward Snapshot Resolver      |   |  - Multi-Grammar Parser    |   |
|   |  - Dynamic Knowledge Links         |   |  - Semantic Module Resolver|   |
|   |  - Sub-50ms Snapshot Time-Travel   |   |  - DAG Cascading Impact    |   |
|   |                                    |   |  - Byte-Level Surgical Pat.|   |
|   |                                    |   |  - Autonomous Tool Registry|   |
|   +-----------------+------------------+   +--------------+-------------+   |
|                     |                                     |                 |
|                     v                                     v                 |
|        [ SQLite (vora_workspace.db) ]     [ SQLite (vora_code_index.db) ]   |
|                                            [ Disk / Git Repository ]        |
+-----------------------------------------------------------------------------+

```

---

## 2. Mind Engine (Knowledge & Version Graph)

The **Mind Engine** provides deterministic time-travel over spatial visual notes.

* **Storage Paradigm:** Checkpoint + Delta hybrid model.
* **Checkpoint Threshold:** Checkpoints are generated automatically at depth intervals to bound the reconstruction cost to $O(K)$ where $K \le 50$.
* **Backward Resolution:** The `SnapshotResolver` traces backwards from the target snapshot to the nearest checkpoint, applying non-overlapping deltas in a single pass.

### Benchmark Metrics (Verified)
* **Dataset:** 10,000 active objects across 50 deep delta snapshots.
* **Database Ingestion & Delta Generation:** `~310.0 ms`
* **Snapshot Resolution Time:** **`19.7 ms`** ($> 500,000$ resolved objects/sec).

---

## 3. Forge Engine (Code Intelligence Core)

The **Forge Engine** provides structured, semantic code reasoning and byte-level surgical modifications for autonomous AI agents.

### 3.1 Parser & Stable Symbol Identity
* Extracts symbols with deterministic hashes (`sym_{hash}`) based on normalized path, scope hierarchy, and symbol kind.
* Captures exact byte boundaries (`start_byte`, `end_byte`) and line offsets.
* Pure grammar extraction via Tree-sitter without string matching.

### 3.2 Code Intelligence Index & Incremental Engine
* Persistent SQLite storage (`code_files`, `code_symbols`, `code_imports`, `code_references`).
* **Incremental Ingestion:** $O(1)$ fast-path check via content hashing. Unchanged files bypass re-parsing entirely.

### 3.3 Semantic Dependency & Module Resolver
* Resolves relative imports (`./math`, `../services/user`) and Rust paths (`crate::*`, `super::*`).
* Maps caller scopes to callee symbol definitions to construct `SemanticProjectGraph`.

### 3.4 Cascading DAG Impact Engine
* Performs breadth-first traversal over the inverse call graph with cycle detection.
* Categorizes blast radius into Direct Dependents, Indirect Dependents, and automatically detects affected test suites (`*.test.ts`, `*_test.rs`).

### 3.5 Byte-Level Surgical Patcher & Pre-Flight Check
* Patches exact byte slices on disk without full-file overwrites.
* Validates AST syntax integrity prior to committing file system changes.

### Benchmark Metrics (Verified)
* **Symbol SQLite Indexing (1,000 Symbols):** `40.6 ms`
* **Incremental Hash Skip ($O(1)$):** **`58.6 µs`**
* **Semantic Graph Construction:** `1.5 ms`
* **Impact Blast Radius Calculation:** **`9.5 µs`**
* **Unit Tests Status:** 5/5 Passed Green.

---

## 4. Autonomous Agent Tool Protocol

| Tool Name | Operation | Safety & Validation |
| :--- | :--- | :--- |
| `find_symbol` | Search symbols across workspace index | SQLite Indexed Lookup |
| `get_symbol_context` | Read exact code slice and related definitions | Token-efficient sub-slice |
| `get_impact_radius` | Calculate cascading dependents & test files | DAG Cycle-Safe BFS |
| `apply_surgical_patch` | Replace exact byte range on disk | Tree-sitter Syntax Check |
| `index_workspace` | Trigger workspace incremental indexing | Content Hash Protected |

---

## 5. Key Operational Shortcuts

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `Ctrl + Shift + A` | Toggle Forge AI Agent Drawer | Global |
| `Ctrl + Shift + H` | Calculate & Paint Git Churn Heatmap | Canvas |
| `Ctrl + Shift + F` | Smooth Focus & Fit Active Graph View | Canvas |
| `Ctrl + Shift + S` | Open Commit Snapshot Modal | Global |
| `Ctrl + /` | Switch Left Panel (Boards / Library) | Global |
| `Ctrl + Z` / `Ctrl + Y` | Multi-Level Canvas Undo / Redo | Canvas |

```
