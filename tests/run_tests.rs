use test_log::test;

use tidploy::{run_command, CommandError, GlobalArguments, RunArguments, StateContext};

#[test]
fn test_run() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    global_args.context = Some(StateContext::None);
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
    global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_spinner.sh".to_owned());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    // \u{8} is backspace, the final rendered output is only '/'
    assert_eq!("\u{8}-\u{8}\\\u{8}|\u{8}/\u{8}-\u{8}\\\u{8}|\u{8}/\u{8}-\u{8}\\\u{8}|\u{8}/", output.out);

    Ok(())
}

/// This test checks whether the stderr and stdout are shown in the correct order.
#[test]
fn test_stdout_stderr() -> Result<(), CommandError> {
    let mut global_args = GlobalArguments::default();
    let mut args = RunArguments::default();
    global_args.context = Some(StateContext::None);
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
    global_args.context = Some(StateContext::None);
    args.executable = Some("examples/run/example_input.sh".to_owned());
    args.input_bytes = Some("foo".into());

    let output = run_command(global_args, args)?;
    assert!(output.exit.success());

    assert_eq!("You entered: foo\n", output.out);

    Ok(())
}