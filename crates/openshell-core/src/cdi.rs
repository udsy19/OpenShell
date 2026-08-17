// SPDX-FileCopyrightText: Copyright (c) 2025-2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared CDI context schema and resolver helpers.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CDI_CONTEXT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CdiContext {
    pub version: u32,
    pub selected_devices: Vec<String>,
    pub spec_dirs: Vec<CdiSpecDirectory>,
}

impl CdiContext {
    #[must_use]
    pub fn new(selected_devices: Vec<String>, spec_dirs: Vec<CdiSpecDirectory>) -> Self {
        Self {
            version: CDI_CONTEXT_VERSION,
            selected_devices,
            spec_dirs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CdiSpecDirectory {
    pub path: String,
    pub source: String,
}

impl CdiSpecDirectory {
    #[must_use]
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CdiDerivedRequirements {
    pub device_node_paths: Vec<String>,
    pub read_only_mount_paths: Vec<String>,
    pub read_write_mount_paths: Vec<String>,
    pub additional_gids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdiPathKind {
    File,
    Directory,
    CharacterDevice,
    BlockDevice,
    Other,
}

impl fmt::Display for CdiPathKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => f.write_str("file"),
            Self::Directory => f.write_str("directory"),
            Self::CharacterDevice => f.write_str("character device"),
            Self::BlockDevice => f.write_str("block device"),
            Self::Other => f.write_str("other"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CdiError {
    #[error("CDI policy resolution is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("failed to read CDI context '{}': {source}", path.display())]
    ContextRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse CDI context '{}': {source}", path.display())]
    ContextParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("unsupported CDI context version {0}")]
    UnsupportedContextVersion(u32),
    #[error("CDI spec dir '{path}' from source '{diagnostic_source}' is unsafe: {reason}")]
    UnsafeSpecDir {
        path: String,
        diagnostic_source: String,
        reason: &'static str,
    },
    #[error("selected CDI device '{0}' was not found in mounted CDI specs")]
    MissingDevice(String),
    #[error("failed to merge CDI edits for '{device}': {error}")]
    EditMerge { device: String, error: String },
    #[error("failed to encode resolved CDI edits: {source}")]
    EditEncode { source: serde_json::Error },
    #[error("failed to decode resolved CDI edits: {source}")]
    EditDecode { source: serde_json::Error },
    #[error("CDI-derived path '{path}' is unsafe: {reason}")]
    UnsafePolicyPath { path: String, reason: &'static str },
    #[error("CDI path '{path}' requested conflicting access modes")]
    ConflictingAccess { path: String },
    #[error(
        "CDI writable mount '{path}' is not explicitly listed in the sandbox policy read_write paths"
    )]
    WritableMountNotAllowed { path: String },
    #[error("CDI writable mount '{path}' must target a single file, found {kind}")]
    WritableMountNotFile { path: String, kind: String },
    #[error("CDI device node '{path}' must target a character or block device, found {kind}")]
    DeviceNodeNotDevice { path: String, kind: String },
    #[error("CDI mount '{path}' has conflicting ro/rw options")]
    ConflictingMountOptions { path: String },
}

pub fn read_context(path: impl AsRef<Path>) -> Result<CdiContext, CdiError> {
    let path = path.as_ref();
    let json = std::fs::read_to_string(path).map_err(|source| CdiError::ContextRead {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&json).map_err(|source| CdiError::ContextParse {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(target_os = "linux")]
#[path = "cdi_linux.rs"]
mod cdi_linux;
#[cfg(target_os = "linux")]
pub use cdi_linux::{
    filesystem_path_kind, resolve_cdi_context, resolve_cdi_context_with_path_kind,
};

#[cfg(not(target_os = "linux"))]
#[path = "cdi_stub.rs"]
mod cdi_stub;
#[cfg(not(target_os = "linux"))]
pub use cdi_stub::resolve_cdi_context;
