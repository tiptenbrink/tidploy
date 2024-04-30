use color_eyre::eyre::{Context, ContextCompat, Report};
use std::io::{stderr, stdout, Read, Write};
use std::process::ExitStatus;
use std::str;
use duct::cmd;
use std::thread::sleep;
use std::time::Duration;
/// This is purely application-level code, hence you would never want to reference it as a library.
/// For this reason we do not really care about the exact errors and need not match on them.
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command as Cmd, Stdio},
};
use tracing::{debug, span, Level};

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

pub struct EntrypointOut {
    pub out: String,
    pub exit: ExitStatus
}

/// Runs the entrypoint, sending the entrypoint's stdout and stderr to stdout
pub(crate) fn run_entrypoint<P: AsRef<Path>>(
    entrypoint_dir: P,
    entrypoint: &str,
    envs: HashMap<String, String>,
) -> Result<EntrypointOut, Report> {
    println!("Running {}!", &entrypoint);
    let program_path = entrypoint_dir.as_ref().join(entrypoint);
    let mut combined_envs: HashMap<_, _> = std::env::vars().collect();
    combined_envs.extend(envs);

    let cmd_expr = cmd(&program_path, Vec::<String>::new())
        .dir(entrypoint_dir.as_ref())
        .full_env(&combined_envs);

    let reader = cmd_expr.stderr_to_stdout().reader()?;
    
    let entry_span = span!(Level::DEBUG, "entrypoint", path = program_path.to_str());
    let _enter = entry_span.enter();

    let mut out: String = String::with_capacity(128);
    
    let mut reader = BufReader::new(reader);
    let mut buffer_out = [0; 32];
    loop {
        let bytes_read_out = reader.read(&mut buffer_out).wrap_err("Error reading stdout bytes!")?;

        if bytes_read_out > 0 {
            let string_buf = str::from_utf8(&buffer_out[..bytes_read_out]).wrap_err("Error converting stdout bytes to UTF-8!")?;
            print!("{}", string_buf);
            // This flush is important in case the script only writes a few characters
            // Like in the case of a progress bar or spinner
            let _ = stdout().flush();
            out.push_str(string_buf);
        }
        else {
            break;
        }
    };
    let inner_reader = reader.into_inner();
    let maybe_output = inner_reader.try_wait().wrap_err("Error trying to get reader exit status!")?;
    let exit = maybe_output.map(|out| out.status).unwrap_or(ExitStatus::default());

    Ok(EntrypointOut {
        out,
        exit
    })
}

// #[cfg(test)]
// mod tests {
//     use std::env;

//     use crate::git::git_root_dir;

//     use super::*;

//     #[test]
//     fn test_run_entrypoint() {
//         let current_dir = env::current_dir().unwrap();
//         let project_dir = git_root_dir(&current_dir).unwrap();
//         let project_path = Path::new(&project_dir).join("examples").join("run");

//         run_entrypoint(project_path, "do_echo.sh", HashMap::new()).unwrap();
//     }

//     #[test]
//     fn test_spawn() {
//         let current_dir = env::current_dir().unwrap();
//         let project_dir = git_root_dir(&current_dir).unwrap();
//         let project_path = Path::new(&project_dir).join("examples").join("run");

//         run_entrypoint(project_path, "do_echo.sh", HashMap::new()).unwrap();
//     }
// }
