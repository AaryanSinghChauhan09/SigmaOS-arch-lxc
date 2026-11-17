//! LXC container management interface for SigmaOS.
//!
//! Provides container lifecycle: create, start, stop, freeze, unfreeze.
//! Simulates LXC operations without actual namespace/cgroup syscalls. Dependency-free.

use std::collections::HashMap;
use std::fmt;

/// Errors for LXC operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LxcError {
    ContainerNotFound(String),
    AlreadyRunning(String),
    NotRunning(String),
    InvalidConfig(String),
    CreateFailed(String),
}

impl fmt::Display for LxcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContainerNotFound(c) => write!(f, "container not found: {c}"),
            Self::AlreadyRunning(c) => write!(f, "container already running: {c}"),
            Self::NotRunning(c) => write!(f, "container not running: {c}"),
            Self::InvalidConfig(m) => write!(f, "invalid config: {m}"),
            Self::CreateFailed(m) => write!(f, "create failed: {m}"),
        }
    }
}

impl std::error::Error for LxcError {}

/// Container state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Stopped,
    Running,
    Frozen,
}

impl ContainerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Running => "running",
            Self::Frozen => "frozen",
        }
    }
}

/// Container configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LxcConfig {
    pub name: String,
    pub rootfs: String,
    pub network: bool,
    pub unprivileged: bool,
    pub memory_limit_mb: u64,
    pub cpu_shares: u32,
}

impl LxcConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rootfs: String::new(),
            network: true,
            unprivileged: false,
            memory_limit_mb: 512,
            cpu_shares: 1024,
        }
    }

    pub fn with_rootfs(mut self, path: impl Into<String>) -> Self {
        self.rootfs = path.into();
        self
    }

    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.memory_limit_mb = mb;
        self
    }

    pub fn validate(&self) -> Result<(), LxcError> {
        if self.name.trim().is_empty() {
            return Err(LxcError::InvalidConfig("name must not be empty".into()));
        }
        if self.memory_limit_mb == 0 {
            return Err(LxcError::InvalidConfig("memory limit must be positive".into()));
        }
        Ok(())
    }
}

/// Container instance.
#[derive(Debug, Clone, PartialEq)]
pub struct LxcContainer {
    pub id: String,
    pub config: LxcConfig,
    pub state: ContainerState,
    pub pid: Option<u32>,
}

/// LXC manager.
#[derive(Debug, Clone, Default)]
pub struct LxcManager {
    containers: HashMap<String, LxcContainer>,
    next_id: u64,
}

impl LxcManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, config: LxcConfig) -> Result<String, LxcError> {
        config.validate()?;
        if self.containers.contains_key(&config.name) {
            return Err(LxcError::AlreadyRunning(config.name.clone()));
        }

        self.next_id += 1;
        let id = format!("{}-{}", &config.name[..3.min(config.name.len())], self.next_id);
        let container = LxcContainer {
            id: id.clone(),
            config: config.clone(),
            state: ContainerState::Stopped,
            pid: None,
        };
        self.containers.insert(config.name.clone(), container);
        Ok(id)
    }

    pub fn start(&mut self, name: &str) -> Result<(), LxcError> {
        let container = self.containers.get_mut(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))?;
        if container.state == ContainerState::Running {
            return Err(LxcError::AlreadyRunning(name.to_string()));
        }
        container.state = ContainerState::Running;
        container.pid = Some(1000 + self.next_id as u32);
        Ok(())
    }

    pub fn stop(&mut self, name: &str) -> Result<(), LxcError> {
        let container = self.containers.get_mut(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))?;
        if container.state == ContainerState::Stopped {
            return Err(LxcError::NotRunning(name.to_string()));
        }
        container.state = ContainerState::Stopped;
        container.pid = None;
        Ok(())
    }

    pub fn freeze(&mut self, name: &str) -> Result<(), LxcError> {
        let container = self.containers.get_mut(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))?;
        if container.state != ContainerState::Running {
            return Err(LxcError::NotRunning(name.to_string()));
        }
        container.state = ContainerState::Frozen;
        Ok(())
    }

    pub fn unfreeze(&mut self, name: &str) -> Result<(), LxcError> {
        let container = self.containers.get_mut(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))?;
        if container.state != ContainerState::Frozen {
            return Err(LxcError::NotRunning(name.to_string()));
        }
        container.state = ContainerState::Running;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), LxcError> {
        let container = self.containers.get(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))?;
        if container.state == ContainerState::Running {
            return Err(LxcError::AlreadyRunning(name.to_string()));
        }
        self.containers.remove(name);
        Ok(())
    }

    pub fn inspect(&self, name: &str) -> Result<&LxcContainer, LxcError> {
        self.containers.get(name)
            .ok_or_else(|| LxcError::ContainerNotFound(name.to_string()))
    }

    pub fn list(&self) -> Vec<&LxcContainer> {
        self.containers.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_container_aaa() {
        // Arrange
        let mut mgr = LxcManager::new();
        let config = LxcConfig::new("test1").with_rootfs("/var/lib/lxc/test1").with_memory_limit(1024);

        // Act
        let id = mgr.create(config);

        // Assert
        assert!(id.is_ok());
        assert_eq!(mgr.list().len(), 1);
    }

    #[test]
    fn test_start_stop_container_aaa() {
        // Arrange
        let mut mgr = LxcManager::new();
        mgr.create(LxcConfig::new("test1")).unwrap();

        // Act
        mgr.start("test1").unwrap();
        let state = mgr.inspect("test1").unwrap().state;
        mgr.stop("test1").unwrap();

        // Assert
        assert_eq!(state, ContainerState::Running);
        assert_eq!(mgr.inspect("test1").unwrap().state, ContainerState::Stopped);
    }

    #[test]
    fn test_freeze_unfreeze_aaa() {
        // Arrange
        let mut mgr = LxcManager::new();
        mgr.create(LxcConfig::new("test1")).unwrap();
        mgr.start("test1").unwrap();

        // Act
        mgr.freeze("test1").unwrap();
        let frozen = mgr.inspect("test1").unwrap().state;
        mgr.unfreeze("test1").unwrap();

        // Assert
        assert_eq!(frozen, ContainerState::Frozen);
        assert_eq!(mgr.inspect("test1").unwrap().state, ContainerState::Running);
    }
}
