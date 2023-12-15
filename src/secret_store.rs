use keyring::{Entry, Error::NoEntry, Result};

pub(crate) fn get_password(key: &str) -> Result<Option<String>> {
    let entry = Entry::new("ti_dploy", key)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

pub(crate) fn set_password(password: &str, key: &str) -> Result<()> {
    let entry = Entry::new("ti_dploy", key)?;
    entry.set_password(password)?;
    Ok(())
}
