# SYSTEM: You are an expert Rust software architect.
Modify the target symbol according to requirements with surgical precision.

## GLOBAL ARCHITECTURAL SKELETON
```rust

// File: src-tauri/benches/load_test.rs
  fn main() {

// File: src-tauri/build.rs
  fn main() {

// File: src-tauri/src/bin/vora_commit.rs
  fn main() -> Result<(), Box<dyn std::error::Error>> {

// File: src-tauri/src/bin/vora_plan.rs
  fn main() -> Result<(), Box<dyn std::error::Error>> {

// File: src-tauri/src/db.rs
  pub struct AppState {
  pub struct NodeInput {
  pub struct EdgeInput {
  pub struct SnapshotSummary {
  pub fn init_db(conn: &Connection) -> Result<()> {
  pub fn save_semantic_snapshot(

// File: src-tauri/src/lib.rs
  pub struct CardDto {
  fn get_all_cards(state: State<AppState>) -> Result<Vec<CardDto>, String> {
  fn save_card_direct(
  fn move_card_to_bin(state: State<AppState>, id: String) -> Result<(), String> {
  fn restore_card_from_bin(state: State<AppState>, id: String) -> Result<(), String> {
  fn commit_snapshot(
  fn resolve_snapshot(
  fn get_all_snapshots(state: State<AppState>) -> Result<Vec<SnapshotSummary>, String> {
  pub fn run() {

// File: src-tauri/src/main.rs
  fn main() {

// File: src-tauri/src/resolver.rs
  pub struct Placement {
  pub struct Link {
  pub struct ResolvedSnapshot {
  pub struct SnapshotResolver;
  pub fn resolve(conn: &Connection, target_snapshot_id: &str) -> Result<ResolvedSnapshot> {

// File: src-tauri/src/vora_git/ast_engine.rs
  pub struct SymbolNode {
  pub struct AstEngine {
  pub fn new_rust() -> Self {
  pub fn extract_symbols(&mut self, source: &[u8]) -> Vec<SymbolNode> {
  fn traverse_nodes(&self, node: Node, source: &[u8], symbols: &mut Vec<SymbolNode>) {

// File: src-tauri/src/vora_git/commit.rs
  pub struct SymbolDelta {
  pub struct FileDelta {
  pub struct CommitRationale {
  pub struct AstCommitPayload {
  pub fn build(changes: &[TrackedChange], rationale: CommitRationale) -> Option<Self> {
  pub fn to_git_message(&self) -> String {

// File: src-tauri/src/vora_git/graph.rs
  pub struct ImpactedTarget {
  pub struct DependencyGraph {
  pub fn new() -> Self {
  pub fn build_from_definitions(&mut self, all_defs: Vec<SymbolDefinition>) {
  pub fn simulate_impact_cascade(&self, target_symbol: &str) -> Vec<ImpactedTarget> {
  fn test_cascade_impact_simulation() {

// File: src-tauri/src/vora_git/patcher.rs
  pub enum PatchError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
  pub struct AstPatcher<'a> {
  pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
  pub fn patch_symbol(&self, symbol_name: &str, new_code: &str) -> Result<(), PatchError> {
  fn test_surgical_patch_dry_run() {

// File: src-tauri/src/vora_git/planner.rs
  pub struct PlanStep {
  pub struct ExecutionPlan {
  pub struct DynamicPlanner<'a> {
  pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
  pub fn create_plan(&self, root_symbol: &str) -> ExecutionPlan {
  fn test_dynamic_planner_generation() {

// File: src-tauri/src/vora_git/project.rs
  pub struct ProjectContext {
  pub fn load_from_repo<P: AsRef<Path>>(
  pub fn simulate_symbol_mutation(&self, symbol_name: &str) -> Vec<ImpactedTarget> {
  pub fn generate_global_skeleton(&self) -> String {
  fn test_project_context_loading() {

// File: src-tauri/src/vora_git/skeleton.rs
  pub struct SymbolDefinition {
  pub struct SkeletonExtractor {
  pub fn new_rust() -> Self {
  pub fn analyze_file(&mut self, file_path: &str, source: &[u8]) -> Vec<SymbolDefinition> {
  fn extract_calls_in_node(&self, node: &Node, source: &[u8]) -> Vec<String> {

// File: src-tauri/src/vora_git/slicer.rs
  pub struct CodeSlice {
  pub struct ScopeSlicer<'a> {
  pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
  pub fn slice_for_symbol(&self, symbol_name: &str) -> Option<CodeSlice> {
  fn test_scope_slicing_real_symbol() {

// File: src-tauri/src/vora_git/ssot.rs
  pub struct GitSSOT {
  pub struct TrackedChange {
  pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, git2::Error> {
  pub fn load_all_tracked_sources(&self) -> Result<Vec<(String, Vec<u8>)>, git2::Error> {
  pub fn get_semantic_changes(&self) -> Result<Vec<TrackedChange>, git2::Error> {
  pub fn stage_and_commit(&self, message: &str) -> Result<Oid, git2::Error> {

```

## EXECUTION PLAN (Target: resolve)
- Step 1: `resolve` in `src-tauri/src/resolver.rs` (Root modification target)

## TARGET CODE SLICE (`resolve`)
```rust
    pub fn resolve(conn: &Connection, target_snapshot_id: &str) -> Result<ResolvedSnapshot> {
        // ۱. پیدا کردن مسیر از اسنپ‌شات به سمت عقب تا رسیدن به نزدیک‌ترین چک‌پوینت
        let mut chain = Vec::new();
        let mut curr = target_snapshot_id.to_string();

        let nearest_checkpoint_id: String;

        loop {
            let mut stmt = conn.prepare(
                "SELECT parent_snapshot_id, is_checkpoint FROM snapshots WHERE id = ?1"
            )?;
            let row = stmt.query_row(params![curr], |r| {
                Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i32>(1)?))
            })?;

            if row.1 == 1 || row.0.is_none() {
                nearest_checkpoint_id = curr;
                break;
            }

            chain.push(curr.clone());
            curr = row.0.unwrap();
        }

        // زنجیره دلتاها باید از قدیمی به جدید اعمال شوند
        chain.reverse();

        // ۲. لود داده‌های ماتریالایز شده چک‌پوینت
        let mut objects: HashMap<String, String> = HashMap::new();
        let mut placements: HashMap<String, Placement> = HashMap::new();
        let mut links: HashMap<String, Link> = HashMap::new();

        {
            let mut stmt = conn.prepare(
                "SELECT object_id, object_version_id FROM checkpoint_objects WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for r in rows {
                let (k, v) = r?;
                objects.insert(k, v);
            }
        }

        {
            let mut stmt = conn.prepare(
                "SELECT object_id, x, y, width, height, rotation, z_index, style FROM checkpoint_placements WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    Placement {
                        x: r.get(1)?,
                        y: r.get(2)?,
                        width: r.get(3)?,
                        height: r.get(4)?,
                        rotation: r.get(5)?,
                        z_index: r.get(6)?,
                        style: r.get(7)?,
                    }
                ))
            })?;
            for r in rows {
                let (k, v) = r?;
                placements.insert(k, v);
            }
        }

        {
            let mut stmt = conn.prepare(
                "SELECT link_id, source_object_id, target_object_id, link_type FROM checkpoint_links WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    Link {
                        source_object_id: r.get(1)?,
                        target_object_id: r.get(2)?,
                        link_type: r.get(3)?,
                    }
                ))
            })?;
            for r in rows {
                let (k, v) = r?;
                links.insert(k, v);
            }
        }

        // ۳. اعمال دلتاها (Delta Coalescing & State Resolution)
        for snap_id in chain {
            // اعمال تغییرات آبجکت‌ها
            let mut stmt_obj = conn.prepare(
                "SELECT object_id, operation, object_version_id FROM snapshot_object_changes WHERE snapshot_id = ?1"
            )?;
            let obj_rows = stmt_obj.query_map(params![snap_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
            })?;

            for r in obj_rows {
                let (obj_id, op, ver_id) = r?;
                match op.as_str() {
                    "ADD" | "UPDATE_VERSION" => {
                        if let Some(v) = ver_id {
                            objects.insert(obj_id, v);
                        }
                    }
                    "REMOVE" => {
                        objects.remove(&obj_id);
                        placements.remove(&obj_id);
                    }
                    _ => {}
                }
            }

            // اعمال تغییرات چیدمان (Placement Deltas)
            let mut stmt_plc = conn.prepare(
                "SELECT object_id, operation, x, y, width, height, rotation, z_index, style FROM snapshot_placement_changes WHERE snapshot_id = ?1"
            )?;
            let plc_rows = stmt_plc.query_map(params![snap_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<f64>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, Option<f64>>(4)?,
                    r.get::<_, Option<f64>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<i32>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                ))
            })?;

            for r in plc_rows {
                let (obj_id, op, x, y, w, h, rot, z, style) = r?;
                if op == "REMOVE" {
                    placements.remove(&obj_id);
                } else if let Some(current) = placements.get_mut(&obj_id) {
                    if let Some(val) = x { current.x = val; }
                    if let Some(val) = y { current.y = val; }
                    if let Some(val) = w { current.width = val; }
                    if let Some(val) = h { current.height = val; }
                    if let Some(val) = rot { current.rotation = val; }
                    if let Some(val) = z { current.z_index = val; }
                    if style.is_some() { current.style = style; }
                } else if op == "ADD" {
                    placements.insert(
                        obj_id,
                        Placement {
                            x: x.unwrap_or(0.0),
                            y: y.unwrap_or(0.0),
                            width: w.unwrap_or(200.0),
                            height: h.unwrap_or(150.0),
                            rotation: rot.unwrap_or(0.0),
                            z_index: z.unwrap_or(0),
                            style,
                        },
                    );
                }
            }
        }

        Ok(ResolvedSnapshot {
            snapshot_id: target_snapshot_id.to_string(),
            objects,
            placements,
            links,
        })
    }
```

## RELATED DEFINITIONS / CALLERS
```rust
// src-tauri/src/resolver.rs (struct_item)
pub struct ResolvedSnapshot {
// src-tauri/src/resolver.rs (struct_item)
pub struct Placement {
// src-tauri/src/resolver.rs (struct_item)
pub struct Link {
```
