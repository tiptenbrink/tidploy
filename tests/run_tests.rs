use std::path::{Path, PathBuf};

use test_log::test;
use keyring::Entry;

use tidploy::{run_command, secret_command, CommandError, GlobalArguments, RunArguments, SecretArguments, StateContext};

#[test]
fn test_run() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    //global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_echo.sh".to_owned());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    let success_str = "Success!".to_owned();
    assert_eq!(output.out.trim(), success_str);

    Ok(())
}

#[test]
fn test_spinner() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    //global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_spinner.sh".to_owned());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    // \u{8} is backspace, the final rendered output is only '/'
    assert_eq!(
        "\u{8}-\u{8}\\\u{8}|\u{8}/\u{8}-\u{8}\\\u{8}|\u{8}/\u{8}-\u{8}\\\u{8}|\u{8}/",
        output.out
    );

    Ok(())
}

/// This test checks whether the stderr and stdout are shown in the correct order.
#[test]
fn test_stdout_stderr() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    //global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_stderr.sh".to_owned());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    assert_eq!("hello1\nhello2\nerr1\nerr2\nhello3\n", output.out);

    Ok(())
}

#[test]
fn test_input() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    //global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_input.sh".to_owned());
    args.input_bytes = Some("foo".into());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    assert_eq!("You entered: foo\n", output.out);

    Ok(())
}

#[test]
fn test_secret_set() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = SecretArguments::default();
    //global_args.context = Some(StateContext::None);
    let pass = "abc".to_owned();
    args.prompt = Some("abc".to_owned());
    global_args.service = Some("tidploy_test_service".to_owned());

    let output = secret_command(global_args, args)?;

    let entry = Entry::new("tidploy_test_service", &output).unwrap();
    let entry_pass = entry.get_password().unwrap();

    entry.delete_password().unwrap();

    assert_eq!(pass, entry_pass);

    Ok(())
}

struct TestEntry {
    entry: Entry,
}

impl Drop for TestEntry {
    fn drop(&mut self) {
        self.entry.delete_password().unwrap();
    }
}

impl TestEntry {
    fn new(service: &str, key: &str, value: &str) -> Self {
        let entry = Entry::new(service, key).unwrap();

        entry.set_password(value).unwrap();

        Self {
            entry
        }
    }
}

#[test]
fn test_secret_get() -> Result<(), CommandError> {
    let pass = "abc".to_owned();
    let key = "key".to_owned();
    let context_root = env!("CARGO_MANIFEST_DIR");
    let context_path = PathBuf::from(context_root);
    let context = context_path.file_name().unwrap().to_str().unwrap();

    let entry_key = format!("{}::tidploy_root::todo_hash:{}", context, key);
    let _entry = TestEntry::new("tidploy_test_service_get", &entry_key, &pass);

    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    global_args.service = Some("tidploy_test_service_get".to_owned());
    //global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_secret.sh".to_owned());
    args.variables = vec![key, "TIDPLOY_SOME_SECRET".to_owned()];

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    assert_eq!(pass, output.out.trim());

    Ok(())
}