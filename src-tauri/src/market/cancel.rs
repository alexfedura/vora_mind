//! v6.3.0f: Per-request cancellation tokens.
//!
//! Frontend sends monotonic `request_id` with each detect call and
//! includes `cancel_previous` for the prior in-flight id. Backend
//! atomically cancels prior THEN registers new. Hot loop checks
//! the token and bails early.
//!
//! NOTE (adaptation): `mkt_*` IPC commands in this codebase are
//! stateless (they open their own DB connection via
//! `open_default_db_connection()` — there is no shared `AppState`
//! for market). Instead of adding a `.manage()` state entry, the
//! registry lives in a process-global `static` behind `OnceLock`.
//! Semantics are identical: cancel-prev + register-new happen
//! atomically at the top of `mkt_detect_boxes`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Global single-flight lock: only one detect runs at a time.
static DETECT_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

pub fn detect_lock() -> &'static tokio::sync::Mutex<()> {
    DETECT_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Newest request id seen by the backend. Queued requests compare
/// against this after acquiring the lock; if a newer one arrived,
/// they bail INSTANTLY without doing any work.
static LATEST_ID: AtomicU64 = AtomicU64::new(0);

pub fn set_latest(id: u64) {
    LATEST_ID.store(id, Ordering::SeqCst);
}
pub fn latest() -> u64 {
    LATEST_ID.load(Ordering::SeqCst)
}

#[derive(Default)]
pub struct CancelRegistry {
    tokens: Mutex<HashMap<u64, Arc<AtomicBool>>>,
}

impl CancelRegistry {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
        }
    }

    /// Register fresh token for `id`. If `id` already exists, it is
    /// REPLACED (frontend guarantees monotonic ids).
    pub fn register(&self, id: u64) -> Arc<AtomicBool> {
        let mut m = self.tokens.lock().unwrap();
        let t = Arc::new(AtomicBool::new(false));
        m.insert(id, t.clone());
        t
    }

    /// Signal cancellation (idempotent — no-op if id unknown).
    pub fn cancel(&self, id: u64) {
        let m = self.tokens.lock().unwrap();
        if let Some(t) = m.get(&id) {
            t.store(true, Ordering::Relaxed);
        }
    }

    /// Cancel ALL registered tokens.
    pub fn cancel_all(&self) {
        let m = self.tokens.lock().unwrap();
        for t in m.values() {
            t.store(true, Ordering::Relaxed);
        }
    }

    /// Remove token (after work is done).
    pub fn remove(&self, id: u64) {
        let mut m = self.tokens.lock().unwrap();
        m.remove(&id);
    }
}

/// Free-function helper — avoids closure borrow issues in hot loops.
#[inline]
pub fn is_cancelled(token: &Option<Arc<AtomicBool>>) -> bool {
    match token {
        Some(t) => t.load(Ordering::Relaxed),
        None => false,
    }
}

/// Process-global registry (see module docs for why global, not State).
pub fn global() -> &'static CancelRegistry {
    static REG: OnceLock<CancelRegistry> = OnceLock::new();
    REG.get_or_init(CancelRegistry::new)
}

/// Alias for global registry (v6.3.0g naming).
pub fn registry() -> &'static CancelRegistry {
    global()
}
