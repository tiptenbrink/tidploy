use keyring::{Entry, Error::NoEntry, Result as KeyringResult};

pub(crate) fn get_password(name: &str, stage: &str) -> KeyringResult<Option<String>> {
    let user = format!("{}_{}", name, stage);
    let entry = Entry::new("ti_dploy", &user)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

pub(crate) fn set_password(password: &str, name: &str, stage: &str) -> KeyringResult<()> {
    let user = format!("{}_{}", name, stage);
    let entry = Entry::new("ti_dploy", &user)?;
    entry.set_password(password)?;
    Ok(())
}
