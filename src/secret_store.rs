use keyring::{Entry, Error::NoEntry, Result as KeyringResult};

pub(crate) fn get_password(stage: &str) -> KeyringResult<Option<String>> {
    let entry = Entry::new("ti_dploy", stage)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

pub(crate) fn set_password(password: &str, stage: &str) -> KeyringResult<()> {
    let entry = Entry::new("ti_dploy", stage)?;
    entry.set_password(password)?;
    Ok(())
}
