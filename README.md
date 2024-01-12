This CLI tool is designed to make it easy to deploy small applications.

Designed for Unix-like deployments. Assumes the existence of `tar` and `git` on your system. Windows file paths might especially be a problem.

## Deploy unit example

### tidploy.toml

```toml
exe_name = "entrypoint.sh"

[[vars]]
env_name = "BWS_ACCESS_TOKEN"
key = "bws"
```

It will run `entrypoint.sh` and try to load the secret with key `bws` and load it as an environment variable named `BWS_ACCESS_TOKEN`. 

## Run examples

If you want to save a globally scoped secret, run:

```tidploy secret <secret name> --context none```

You can scope it to a repository URL by adding `-r <repository url>` to the command.

If you then have a file called `abc.sh`:

```
#!/bin/bash

echo $ABCD
```

And you run:

```
tidploy run --context none -x abc.sh -v <secret name> ABCD
```

The file will be run and print the value you provided to the secret.

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