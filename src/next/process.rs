/// This is purely application-level code, hence you would never want to reference it as a library. 
/// For this reason we do not really care about the exact errors and need not match on them.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command as Cmd, Stdio},
};
use tracing::{span, Level};
use color_eyre::eyre::{Context, ContextCompat, Report};
use std::str;
use crate::errors::{ProcessError, ProcessErrorKind};

/// Read output bytes into a string and trim any whitespace at the end.
fn process_out(bytes: Vec<u8>) -> Result<String, Report> {
    let mut output_string = String::from_utf8(bytes)
        .wrap_err("Error occurred decoding process output bytes as UTF-8.")?;
    // We use truncate to prevent having to copy the string, which could be quite large as it's
    // the output of a whole program
    let trim_len = output_string.trim_end().len();
    output_string.truncate(trim_len);
    
    Ok(output_string)
}


pub(crate) fn run_entrypoint<P: AsRef<Path>>(
    entrypoint_dir: P,
    entrypoint: &str,
    envs: HashMap<String, String>,
) -> Result<(), Report> {
    println!("Running {}!", &entrypoint);
    let program_path = entrypoint_dir.as_ref().join(entrypoint);
    let entry_span = span!(Level::DEBUG, "entrypoint", path = program_path.to_str());
    let _enter = entry_span.enter();
    let mut entrypoint_output = Cmd::new(&program_path)
        .current_dir(&entrypoint_dir)
        .envs(&envs)
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err("System IO error occurred spawning process!")?;

    let entrypoint_stdout = entrypoint_output
        .stdout
        .take()
        .wrap_err("No output for process!")?;

    let reader = BufReader::new(entrypoint_stdout);

    reader
        .lines()
        .map_while(Result::ok)
        .for_each(|line| println!("{}", line));

    let output_stderr = entrypoint_output
        .wait_with_output()
        .wrap_err("Error reading output stderr!")?
        .stderr;
    if !output_stderr.is_empty() {
        println!(
            "Entrypoint {:?} failed with error: {}",
            program_path.as_path(),
            process_out(output_stderr)?
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