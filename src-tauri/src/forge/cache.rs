use super::model::ContentHash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedSymbolEntry {
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileSemanticCacheEntry {
    pub content_hash: String,
    pub mtime_millis: u64,
    pub symbols: Vec<CachedSymbolEntry>,
    pub vector_offset: Option<u64>,
    pub vector_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticCacheManifest {
    pub version: u32,
    pub last_synced_at: u64,
    pub files: HashMap<String, FileSemanticCacheEntry>,
}

pub struct SemanticCacheManager {
    cache_dir: PathBuf,
    manifest_path: PathBuf,
    vectors_path: PathBuf,
    manifest: SemanticCacheManifest,
}

impl SemanticCacheManager {
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> std::io::Result<Self> {
        let dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let manifest_path = dir.join("semantic_manifest.json");
        let vectors_path = dir.join("semantic_vectors.bin");

        let mut manager = Self {
            cache_dir: dir,
            manifest_path,
            vectors_path,
            manifest: SemanticCacheManifest::default(),
        };

        manager.load_manifest()?;
        Ok(manager)
    }

    /// بارگذاری آنی مانیفست از دیسک
    pub fn load_manifest(&mut self) -> std::io::Result<()> {
        if !self.manifest_path.exists() {
            self.manifest = SemanticCacheManifest {
                version: 1,
                last_synced_at: 0,
                files: HashMap::new(),
            };
            return Ok(());
        }

        let mut file = File::open(&self.manifest_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match serde_json::from_str::<SemanticCacheManifest>(&content) {
            Ok(manifest) => {
                self.manifest = manifest;
                Ok(())
            }
            Err(_) => {
                self.manifest = SemanticCacheManifest {
                    version: 1,
                    last_synced_at: 0,
                    files: HashMap::new(),
                };
                Ok(())
            }
        }
    }

    /// اعتبارسنجی سریع: آیا فایل نسبت به کش تغییر کرده است؟
    pub fn is_file_up_to_date(&self, file_path: &str, current_mtime: u64, content: &str) -> bool {
        if let Some(entry) = self.manifest.files.get(file_path) {
            if entry.mtime_millis == current_mtime {
                return true;
            }
            let current_hash = ContentHash::from_str(content).0;
            return entry.content_hash == current_hash;
        }
        false
    }

    /// بازیابی نمادهای کش‌شده یک فایل
    pub fn get_cached_symbols(&self, file_path: &str) -> Option<&Vec<CachedSymbolEntry>> {
        self.manifest
            .files
            .get(file_path)
            .map(|entry| &entry.symbols)
    }

    /// به‌روزرسانی کش یک فایل
    pub fn update_file_cache(
        &mut self,
        file_path: &str,
        content: &str,
        mtime_millis: u64,
        symbols: Vec<CachedSymbolEntry>,
    ) {
        let content_hash = ContentHash::from_str(content).0;
        let entry = FileSemanticCacheEntry {
            content_hash,
            mtime_millis,
            symbols,
            vector_offset: None,
            vector_length: None,
        };
        self.manifest.files.insert(file_path.to_string(), entry);
    }

    /// ذخیره‌سازی باینری آرایه‌های برداری (Float32Array) با دسترسی مستقیم
    pub fn append_vector(&mut self, file_path: &str, vector: &[f32]) -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.vectors_path)?;

        let offset = file.metadata()?.len();

        let byte_slice = unsafe {
            std::slice::from_raw_parts(vector.as_ptr() as *const u8, std::mem::size_of_val(vector))
        };

        file.write_all(byte_slice)?;

        if let Some(entry) = self.manifest.files.get_mut(file_path) {
            entry.vector_offset = Some(offset);
            entry.vector_length = Some(vector.len());
        }

        Ok(())
    }

    /// خواندن باینری بردار بر اساس آفست دیسک
    pub fn read_vector(&self, file_path: &str) -> std::io::Result<Option<Vec<f32>>> {
        let entry = match self.manifest.files.get(file_path) {
            Some(e) => e,
            None => return Ok(None),
        };

        if let (Some(offset), Some(len)) = (entry.vector_offset, entry.vector_length) {
            let mut file = File::open(&self.vectors_path)?;
            use std::io::Seek;
            file.seek(std::io::SeekFrom::Start(offset))?;

            let mut byte_buffer = vec![0u8; len * std::mem::size_of::<f32>()];
            file.read_exact(&mut byte_buffer)?;

            let mut float_buffer = vec![0.0f32; len];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    byte_buffer.as_ptr(),
                    float_buffer.as_mut_ptr() as *mut u8,
                    byte_buffer.len(),
                );
            }

            return Ok(Some(float_buffer));
        }

        Ok(None)
    }

    /// ذخیره‌سازی اتمیک مانیفست روی دیسک (Atomic Flush)
    pub fn flush_to_disk(&mut self) -> std::io::Result<()> {
        self.manifest.last_synced_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let encoded =
            serde_json::to_string_pretty(&self.manifest).map_err(std::io::Error::other)?;

        let temp_path = self.cache_dir.join("semantic_manifest.tmp");
        {
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(encoded.as_bytes())?;
            temp_file.sync_all()?;
        }

        fs::rename(temp_path, &self.manifest_path)?;
        Ok(())
    }
}
