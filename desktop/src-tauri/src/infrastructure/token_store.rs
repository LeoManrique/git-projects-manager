use crate::domain::auth::SyncSession;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

const SERVICE: &str = "git-projects-manager";
const ACCOUNT: &str = "sync_session";

pub trait TokenStore: Send + Sync {
    fn load(&self) -> Result<Option<SyncSession>>;
    fn save(&self, session: &SyncSession) -> Result<()>;
    fn clear(&self) -> Result<()>;
}

pub struct KeyringTokenStore;

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<SyncSession>> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        match entry.get_password() {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn save(&self, session: &SyncSession) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        let json = serde_json::to_string(session)?;
        entry.set_password(&json)?;
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub struct FileTokenStore {
    path: PathBuf,
}

impl FileTokenStore {
    pub fn new() -> Result<Self> {
        let dir = dirs::config_dir()
            .context("could not find config directory")?
            .join(".git-projects-manager");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { path: dir.join("session.json") })
    }
}

impl TokenStore for FileTokenStore {
    fn load(&self) -> Result<Option<SyncSession>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&self.path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    fn save(&self, session: &SyncSession) -> Result<()> {
        let content = serde_json::to_string(session)?;
        std::fs::write(&self.path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.path, perms)?;
        }
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

/// Probe the keyring; if it's usable on this system, prefer it. Otherwise
/// fall back to a 0600-mode JSON file in the app config directory.
pub fn default_store() -> Arc<dyn TokenStore> {
    if keyring::Entry::new(SERVICE, ACCOUNT).is_ok() {
        return Arc::new(KeyringTokenStore);
    }
    match FileTokenStore::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::error!(?e, "no usable token store; sync sessions will not persist");
            Arc::new(NullTokenStore)
        }
    }
}

struct NullTokenStore;

impl TokenStore for NullTokenStore {
    fn load(&self) -> Result<Option<SyncSession>> { Ok(None) }
    fn save(&self, _: &SyncSession) -> Result<()> { Ok(()) }
    fn clear(&self) -> Result<()> { Ok(()) }
}
