use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command as Cmd, Stdio},
};

use crate::errors::{ProcessError, ProcessErrorKind};

pub(crate) fn process_out(bytes: Vec<u8>, err_msg: String) -> Result<String, ProcessError> {
    Ok(String::from_utf8(bytes)
        .map_err(|e| ProcessError {
            msg: err_msg,
            source: ProcessErrorKind::Decode(e),
        })?
        .trim_end()
        .to_owned())
}

/// Convenience function to create a process error.
fn err_ctx<P: AsRef<Path>>(e: impl Into<ProcessErrorKind>, info: &str, p: P) -> ProcessError {
    let msg = format!(
        "IO error {} (running entrypoint at path: {:?})",
        info,
        p.as_ref()
    );
    ProcessError {
        msg,
        source: e.into(),
    }
}

pub(crate) fn run_entrypoint<P: AsRef<Path>>(
    entrypoint_dir: P,
    entrypoint: &str,
    envs: HashMap<String, String>,
) -> Result<(), ProcessError> {
    println!("Running {}!", &entrypoint);
    let program_path = entrypoint_dir.as_ref().join(entrypoint);
    let mut entrypoint_output = Cmd::new(&program_path)
        .current_dir(&entrypoint_dir)
        .envs(&envs)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| err_ctx(e, "spawning process", &program_path))?;

    let entrypoint_stdout = entrypoint_output
        .stdout
        .take()
        .ok_or_else(|| err_ctx(ProcessErrorKind::NoOutput, "", &program_path))?;

    let reader = BufReader::new(entrypoint_stdout);

    reader
        .lines()
        .map_while(Result::ok)
        .for_each(|line| println!("{}", line));

    let output_stderr = entrypoint_output
        .wait_with_output()
        .map_err(|e| err_ctx(e, "reading output err", &program_path))?
        .stderr;
    if !output_stderr.is_empty() {
        println!(
            "Entrypoint {:?} failed with error: {}",
            program_path.as_path(),
            process_out(output_stderr, "Failed to decode entrypoint".to_owned())?
        )
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::git::git_root_dir;

    use super::*;

    #[test]
    fn test_run_entrypoint() {
        let current_dir = env::current_dir().unwrap();
        let project_dir = git_root_dir(&current_dir).unwrap();
        let project_path = Path::new(&project_dir).join("examples").join("run");

        run_entrypoint(project_path, "do_echo.sh", HashMap::new()).unwrap();
    }

    #[test]
    fn test_spawn() {
        let current_dir = env::current_dir().unwrap();
        let project_dir = git_root_dir(&current_dir).unwrap();
        let project_path = Path::new(&project_dir).join("examples").join("run");

        run_entrypoint(project_path, "do_echo.sh", HashMap::new()).unwrap();
    }
}
