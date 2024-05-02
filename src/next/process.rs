use color_eyre::eyre::{Context, Report};
use duct::{cmd, IntoExecutablePath};
use relative_path::RelativePath;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::io::{stdout, Read, Write};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::str;
use std::{collections::HashMap, io::BufReader, path::Path};
use tracing::{span, Level};

use super::errors::{ProcessError, ProcessIOError};

pub struct EntrypointOut {
    pub out: String,
    pub exit: ExitStatus,
}

pub(crate) fn process_out(bytes: Vec<u8>, info: String) -> Result<String, ProcessError> {
    Ok(String::from_utf8(bytes)
        .map_err(|_e| ProcessError::Decode(info))?
        .trim_end()
        .to_owned())
}

pub(crate) fn process_complete_output<P, E, S>(
    working_dir: P,
    program: E,
    args: Vec<S>,
) -> Result<EntrypointOut, ProcessError>
where
    // This is pretty bad...
    P: Into<PathBuf> + Debug + Clone,
    E: IntoExecutablePath + Debug + Clone,
    S: AsRef<OsStr> + Debug,
{
    let output = cmd(program.clone(), &args)
        .dir(working_dir.clone())
        .stderr_to_stdout()
        .stdout_capture()
        .unchecked()
        .run()
        .map_err(|e| ProcessIOError {
            msg: format!(
                "Process {:?} with args {:?} failed to run in {:?}",
                program, args, working_dir
            ),
            source: e,
        })?;

    let out = process_out(output.stdout, "stdout".to_owned())?;

    Ok(EntrypointOut {
        out,
        exit: output.status,
    })
}

/// Runs the entrypoint, sending the entrypoint's stdout and stderr to stdout. It adds the provided envs to
/// the envs of the tidploy process. `input_bytes` is useful mostly for testing, if set to None then the
/// child process will just inherit the stdin of the tidploy process.
pub(crate) fn run_entrypoint<P: AsRef<Path>>(
    entrypoint_dir: P,
    entrypoint: &RelativePath,
    envs: HashMap<String, String>,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    println!("Running {}!", &entrypoint);
    let program_path = entrypoint.to_path(entrypoint_dir.as_ref());
    // Use parent process env variables as base
    let mut combined_envs: HashMap<_, _> = std::env::vars().collect();
    combined_envs.extend(envs);

    let cmd_expr = cmd(&program_path, Vec::<String>::new())
        .dir(entrypoint_dir.as_ref())
        .full_env(&combined_envs)
        .stderr_to_stdout()
        .unchecked();

    // This is useful for testing input
    let cmd_expr = if let Some(input_bytes) = input_bytes {
        cmd_expr.stdin_bytes(input_bytes)
    } else {
        cmd_expr
    };

    let reader = cmd_expr.reader()?;

    let entry_span = span!(Level::DEBUG, "entrypoint", path = program_path.to_str());
    let _enter = entry_span.enter();

    let mut out: String = String::with_capacity(128);

    let mut reader = BufReader::new(reader);
    let mut buffer_out = [0; 32];
    loop {
        let bytes_read_out = reader
            .read(&mut buffer_out)
            .wrap_err("Error reading stdout bytes!")?;

        if bytes_read_out > 0 {
            let string_buf = str::from_utf8(&buffer_out[..bytes_read_out])
                .wrap_err("Error converting stdout bytes to UTF-8!")?;
            print!("{}", string_buf);
            // This flush is important in case the script only writes a few characters
            // Like in the case of a progress bar or spinner
            let _ = stdout().flush();
            out.push_str(string_buf);
        } else {
            break;
        }
    }
    let inner_reader = reader.into_inner();
    let maybe_output = inner_reader
        .try_wait()
        .wrap_err("Error trying to get reader exit status!")?;
    let exit = maybe_output
        .map(|out| out.status)
        .unwrap_or(ExitStatus::default());

    Ok(EntrypointOut { out, exit })
}
