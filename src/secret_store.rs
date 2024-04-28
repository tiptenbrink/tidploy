use keyring::{Entry, Error::NoEntry, Result};
use tracing::debug;

pub(crate) fn get_password(key: &str) -> Result<Option<String>> {
    debug!(
        "Trying to get keyring password with key {} for service ti_dploy",
        key
    );
    let entry = Entry::new("ti_dploy", key)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(NoEntry) => {
            debug!("No entry found!");
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

pub(crate) fn set_password(password: &str, key: &str) -> Result<()> {
    let entry = Entry::new("ti_dploy", key)?;
    entry.set_password(password)?;
    Ok(())
}
