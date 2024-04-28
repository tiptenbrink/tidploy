This CLI tool makes it easy to deploy small applications, specifically targeting hobbyists, organizations and small businesses hosting their application on a single machine.

It is designed to solve a simple problem:

> I have a versioned script I have to run, and I want to provide it some environment variables.

How do you solve this? Well, the simplest solution is each time you want to run the script, you download a new version, you manually load in the environment variables you need into your shell, and then you call the script.

There's some problems with that solution:

- Having to get the right version of the script from the right place to the right place can be annoying.
- The environment variables might be tedious to load in, or you don't want to just write them in your shell and save them in your history. Or maybe they don't change very often and you don't want to type them in each time. Or, you want an automated action to call your script without having to provide it with the environment variable/secret.

`tidploy` aims to solve exactly these problems. Its completely neutral about what you want to use the environment variables for and what kind of script you want to run. However, it has of course been designed to solve it for a specific system, so understanding this system can help you understand `tidploy`, which is explained in [this section](#historyoriginal-goal). 

## Quickstart

### Saving a secret

Save a globally-scoped secret:

```tidploy secret <secret name> --context none```

You can scope it to a repository URL by adding `-r <repository url>` to the command. Or you can simply call the command while in a git repository:

```tidploy secret <secret name>```

It then automatically scopes it to the git repository you are in.

### Running a script

If you have a file called `abc.sh`:

```
#!/bin/bash

echo $ABCD
```

And you run (in the same directory as the file):

```
tidploy run --context none -x abc.sh -v <secret name> ABCD
```

The file will be run and print the value you provided to the secret. Note that if the secret was scoped to the repo it will not find it in this case.

However, if you run

```
tidploy run --context none -r https://github.com/tiptenbrink/tidploy.git -x abc.sh -v <secret name> ABCD
```

it will use the globally-scoped secret we set with the first command if you didn't set a repo-scoped one, even if we did provide it with a repo URL this time.

### Simple deploy unit

Imagine we have the following structure:

```
.
├── entrypoint.sh
└── tidploy.toml
```

Our `tidploy.toml` looks like this:

```toml
exe_name = "entrypoint.sh"

[[vars]]
env_name = "BWS_ACCESS_TOKEN"
key = "bws"
```

If we now run:

```tidploy run```

It will run `entrypoint.sh` and try to load the secret with key `bws` and load it as an environment variable named `BWS_ACCESS_TOKEN`. 

## Troubleshooting

### `run`

Your executable must either be an actual executable file, or it must have a shebang that indicates how to run it. So if you are getting `Exec format error`, try adding `#!/bin/sh` or any other relevant shebang to the top of your file.

## Known issues

* git-local context doesn't function fully. If set without a CLI-set repo, it will change the repo name to the cache repo name dir
* Unstable, no tests yet

## Notes

### History/original goal

I designed `tidploy` as I wanted to streamline the process of deploying a simple Docker Compose application which relied on a single master secret (originally to decrypt a gpg file, later an access token for Bitwarden Secrets Manager). I didn't want to type in this secret each time and also preferred not to have to set up a way to safely share it with my team members. I tried to use Python's [keyring](https://github.com/jaraco/keyring) to store it safely on something like macOS Keychain or Windows Credential Locker. However, I could just not get it to work with the Secret Service API on headless Linux systems (like WSL or the server we actually deployed on...). 

Then came [`keyring-rs`](https://github.com/hwchen/keyring-rs), which allows you to use the Linux kernel's keyutils as a backend, which is perfect for long-living servers that rarely reboot. It does loos its state on reboot, but that's totally fine for our purposes. It works perfectly on headless systems as well! Since it was written in Rust, I wanted to develop the full CLI in Rust as well. Thus, `tidploy` was born. In the end, other problems I came across, like wanting to easily use multiple versions (allowing easy rollback), naturally fit into the same tool.

### Portability

It has been mostly designed for Unix-like systems. `tidploy` assumes the existence of `tar` and `git` (available on your path with exactly those names). It also relies on [`keyring-rs`](https://github.com/hwchen/keyring-rs) in a configuration that means it only supports Linux, macOS and Windows. Furthermore, Windows file paths might be a problem in some cases.

Future:
- tar will probably be removed as a command-line dependency and a platform-agnostic crate will be used instead
- If libgit2 or any other git bindings (gitoxide maybe, see [this](https://github.com/Byron/gitoxide/issues/1046) and [this](https://github.com/Byron/gitoxide/issues/562)) ever support modern git features like partial clone, sparse clone and sparse-checkout, we will be able to remove the dependency on calling git as an external process

## Help

```text
Simple deployment tool for deploying small applications and loading secrets

Usage: tidploy [OPTIONS] <COMMAND>

Commands:
  secret    Save secret with key until reboot
  download  Download tag or version with specific env, run automatically if using deploy
  deploy    Deploy tag or version with specific env
  run       Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
  help      Print this message or the help of the given subcommand(s)

Options:
      --context <CONTEXT>        Contexts other than git-remote (default) are not fully supported [possible values: none, git-remote, git-local]
  -r, --repo <REPO>              Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                The git reference (commit or tag) to use
  -d, --deploy-pth <DEPLOY_PTH>  The path inside the repository that should be used as the primary config source
  -h, --help                     Print help
  -V, --version                  Print version
```

### Save secret

```text
Save secret with key until reboot

Usage: tidploy secret [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --context <CONTEXT>        Contexts other than git-remote (default) are not fully supported [possible values: none, git-remote, git-local]
  -r, --repo <REPO>              Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                The git reference (commit or tag) to use
  -d, --deploy-pth <DEPLOY_PTH>  The path inside the repository that should be used as the primary config source
  -h, --help                     Print help
```


### Download

NOTE: This command is not fully functioning.

```text
Download tag or version with specific env, run automatically if using deploy

Usage: tidploy download [OPTIONS]

Options:
      --repo-only                
      --context <CONTEXT>        Contexts other than git-remote (default) are not fully supported [possible values: none, git-remote, git-local]
  -r, --repo <REPO>              Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                The git reference (commit or tag) to use
  -d, --deploy-pth <DEPLOY_PTH>  The path inside the repository that should be used as the primary config source
  -h, --help                     Print help
```


### Deploy

```text
Deploy tag or version with specific env

Usage: tidploy deploy [OPTIONS]

Options:
  -x, --exe <EXECUTABLE>          
      --no-create                 Don't clone a fresh repository. Will fail if it does not exist. WARNING: The repository might not be up-to-date
  -v <VARIABLES> <VARIABLES>      Variables to load. Supply as many pairs of <key> <env var name> as needed
      --context <CONTEXT>         Contexts other than git-remote (default) are not fully supported [possible values: none, git-remote, git-local]
  -r, --repo <REPO>               Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                 The git reference (commit or tag) to use
  -d, --deploy-pth <DEPLOY_PTH>   The path inside the repository that should be used as the primary config source
  -h, --help                      Print help
```

### Run

```text
Run an entrypoint or archive created by download/deploy and load secrets

Usage: tidploy run [OPTIONS]

Options:
  -x, --exe <EXECUTABLE>          
  -v <VARIABLES> <VARIABLES>      Variables to load. Supply as many pairs of <key> <env var name> as needed
      --archive <ARCHIVE>         Give the exact name of the archive using the format: <repo name final path element without extension>_<commit sha>_<base64url-encoded url without name>
      --context <CONTEXT>         Contexts other than git-remote (default) are not fully supported [possible values: none, git-remote, git-local]
  -r, --repo <REPO>               Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                 The git reference (commit or tag) to use
  -d, --deploy-pth <DEPLOY_PTH>   The path ins
```

## Architecture

`tidploy` does 3 things:
- Parse configuration
  - Here it offers first class support for using OS APIs to safely read and load secrets
- Download and isolate repositories using Git
- Inject configuration into an executable using environment variables

It's important to realize that the only step that surely happens once is the last one, at least in a single run of `tidploy` (you can always call tidploy again from your executable). Before we can run the executable, we need to build up a 'state' that includes all the configuration we want to provide to our executable in the form of environment variables.

Remember, we don't want to manually provide a list of environment variables every time we want to restart our application. Furthermore, this configuration also doesn't just live in one big `.env` file checked into the repository. In particular, we might have secret values or the configuration lives in multiple files. A complex example use case we want to support is the following:

- The current latest commit of our repository (`1abcdef`) is version Y of our application
- We want to restart our application, which in production is still running version X
- Our repository at commit `1abcdef` has some updates to some infrastructure scripts we might want to use, but we made them point to version X as the latest production version (maybe you added `tidploy`)
- We now want to check out commit `1abcdef` on our production server and run `tidploy ...` such that it downloads the repository at the commit corresponding to version X, loads in the correct secrets (corresponding to version X, which might have been changed in the latest commit) and then runs the executable starting the application

This requires the following work on `tidploy`'s side:
- Parse a configuration file in the latest repo saying to download version X
- Download it and put the repository somewhere 
- Parse the configuration of the repository in that older state to locate the executable and which secrets to load
- Run it with all the correct environment variables

As you can see, we now have to parse the configuration twice! This could technically happen even more times, but each time we are building a final state closer to the rich state that allows us to actually run the application.

### State creation