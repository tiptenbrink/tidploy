This CLI tool is designed to make it easy to deploy a Docker Compose application (althoug)

## Help
```
Simple deployment tool for deploying small deploy units and loading secrets

Usage: tidploy <COMMAND>

Commands:
  download  Download tag or version with specific env
  deploy    Deploy tag or version with specific env
  auth      Save authentication details for specific stage until reboot
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Download

```
Download tag or version with specific env

Usage: tidploy download <ENV> [GIT_REF]

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
  -h, --help
          Print help (see a summary with '-h')
```


### Auth

```
Save authentication details for specific stage until reboot

Usage: tidploy auth <STAGE>

Arguments:
  <STAGE>
          Possible values:
          - download: Download stage
          - deploy:   Deploy stage

Options:
  -h, --help
          Print help (see a summary with '-h')
```


### Deploy

```
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
  -l, --latest
          Whether to get the latest version of the ref (default: true)

  -r, --recreate
          Whether to recreate the database (default: false)

  -h, --help
          Print help (see a summary with '-h')
```