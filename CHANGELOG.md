# Changelog

## 0.16.0 2024-05-16

* Significant refactor of how resolving works for run/deploy (secrets still needs to be changed). "state_root" no longer exists. An address always contains a target path, which is the state path. This state path is also what will be used to resolve the config. State resolution is no longer first merged and then resolved, but resolved only at the state path location. Argument resolution happens from the resolve root to the state path. The biggest change here is that relative paths in `tidploy.toml` are now resolved relative to themselves and not relative to the (unknown) resolve root, as this is a more natural API. This is also more similar to how things worked pre-0.14. However, execution path still defaults to the resolve root, but this can be modified by adding `execution_path = "."`.
* Local git cloning now works as it should

## 0.15.0 2024-05-07

* Address system is now used to replace StateIn/StatePaths, deploy now also works similar to before
* A lot of the arguments for the CLI have been changed, so incompatible with 0.14. From now on follows the route to stabilization, as we are basically feature-complete

## 0.14.0 2024-05-05

* Very large update that includes a complete rewrite, currently gated behind `tidploy next`. v1 should hopefully make this the default.
* There is no backwards compatibility with the portion behind `next`, nearly all options have been changed. State resolution is quite different now, with actual loops that try to converge
* There is an "address" system that replaces the way download previously worked
* No more archives, this didn't really add anything
* Git tags are handled quite differently now, we use ls-remote to find the correct commit, they are also cached and stored differently
* Added testing and examples
* Start using (color-)eyre which should print the errors in a nicer format and makes especially the more application-level IO errors a lot easier to handle. This also includes a spantrace and a location, which makes it much easier to track where the error actually occurred. 

## 0.13.1 2024-03-16

* Small change for match in get secret in secret_store, hopefully fixes strange bug

## 0.13.0 2024-01-13

* Update keyring-rs, more explicitly state platform support. Linux has full support, macOS and Windows are untested but could work.
* Use cache dir to save downloads, use tmp only for the extract target location
* Fix config path traversal
* More consistent deploy path/root path usage
* Git local should work now as a context (not yet fully tested)

## 0.12.1 2024-01-12

* Upload changelog

## 0.12.0 2024-01-12

Significant improvements have been made. Debug logs are available when running with `RUST_LOG=DEBUG` set as an environment variable. 

* Support for cloning from local repositories (almost any Git URL should work now). This also separates some more logic.
* `run` now works with a deploy path when running an archive.
* `network` option has been removed, the context now determines this (although it's not yet implemented). 

## 0.11.2 2023-12-15

* Fix exe name being overwritten by default value from CLI. Fix run state precedence.

## 0.11.1 2023-12-15

* Upload changelog.

## 0.11.0 2023-12-15

* Deploy and run function added back in. Not compatible with previous versions. 

## 0.10.0 2023-12-15

* Most features have been temporarily removed during the refactor. Use v0.9.0 if they are needed. This release contains:
    - A major refactor of how state is loaded, to streamline it across commands
    - A revamped "auth" command, which now uses a key and by default scopes only to the repo
    - A revamped "run" command, which now utilizes the general set state

## 0.9.0 2023-12-14

* dployer is the default option now, dployer_env renamed to env_var and new "run" subcommand

## 0.8.0 2023-12-12

* Add support for dployer, which allows secret loading to also be handled externally by using named pipe communication with Docker containers

## 0.7.0 2023-12-11

* Set default repo to default_git_root_origin and check for git_root_origin in order to read URL from current repo

## 0.6.0 - 2023-12-11

* Rewrite Git history to fix commit author
* Remove debug URL print

## 0.5.0 - 2023-12-11

* Separated primitive errors into file
* Fixed extract not having correct path
* Fixed setting password not getting correct name from repo URL

## 0.4.0 - 2023-12-11

