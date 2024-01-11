This CLI tool is designed to make it easy to deploy small applications.

Only works on Linux. 

NOTE: The documentation is currently outdated, but is mostly accurate for v0.8.0 and v0.9.0.

## Deploy unit example

### tidploy.json

```jsonc
{
    # optional, defaults to false
    "dployer": true,
    "info": {
        "latest": "main"
    },
    "secrets": {
        # required if dployer set to true, otherwise optional
        "dployer_env": "BWS_ACCESS_TOKEN",
        "ids": [
            "<secret id>",
            "<secret id>",
        ]
    }
}

```

### With dployer

**Required files**

* dployer.sh
* tidploy.json/toml

The password set for the `deploy` stage is passed as an environment variable with key equal to `secrets.deployer_env`.

### With Bitwarden Secrets Manager integration

**Required files**

* entrypoint.sh
* tidploy.json/toml

If dployer is not specified, it is assumed that the set password for the `deploy` stage is the `BWS_ACCESS_TOKEN`. Secrets in `secrets.ids` will be loaded and passed to the `entrypoint.sh` as environment variables.

## Help

```text
Simple deployment tool for deploying small deploy units and loading secrets

Usage: tidploy [OPTIONS] <COMMAND>

Commands:
  auth      Save authentication details for specific stage until reboot
  download  Download tag or version with specific env, run automatically if using deploy
  deploy    Deploy tag or version with specific env
  run       Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
  help      Print this message or the help of the given subcommand(s)

Options:
      --context <CONTEXT>        [possible values: none, git]
      --network <NETWORK>        [possible values: true, false]
  -r, --repo <REPO>              Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                
  -d, --deploy-pth <DEPLOY_PTH>  
  -h, --help                     Print help
  -V, --version                  Print version
```

### Auth

```text
Save authentication details for specific stage until reboot

Usage: tidploy auth [OPTIONS] <KEY>

Arguments:
  <KEY>  

Options:
      --context <CONTEXT>        [possible values: none, git]
      --network <NETWORK>        [possible values: true, false]
  -r, --repo <REPO>              Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it. Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url' For infering, it looks at the URL set to the 'origin' remote
  -t, --tag <TAG>                
  -d, --deploy-pth <DEPLOY_PTH>  
  -h, --help                     Print help
```


### Download

```text
Download tag or version with specific env

Usage: tidploy download [OPTIONS] <ENV> [GIT_REF]

Arguments:
  <ENV>
          Environment

          Possible values:
          - localdev:   Local development environment
          - staging:    Staging environment
          - production: Production environment

  [GIT_REF]
          Version or tag to download

Options:
  -r, --repo <REPO>
          Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set. Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
          
          [default: default_git_root_origin]

  -h, --help
          Print help (see a summary with '-h')
```


### Deploy

```text
Deploy tag or version with specific env

Usage: tidploy deploy [OPTIONS] <ENV> [GIT_REF]

Arguments:
  <ENV>
          Environment

          Possible values:
          - localdev:   Local development environment
          - staging:    Staging environment
          - production: Production environment

  [GIT_REF]
          Version or tag to deploy. Omit to deploy latest for env

Options:
  -r, --repo <REPO>
          Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set. Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
          
          [default: default_git_root_origin]

  -l, --latest
          Whether to get the latest version of the ref (default: true)

  -c, --recreate
          Whether to recreate the database (default: false)

  -h, --help
          Print help (see a summary with '-h')
```