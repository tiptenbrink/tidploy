### Running an executable

The most simple way to use `tidploy` is to just run some other executable. While this is not something that actually uses any features from `tidploy`, it should still just work and any issues with it mean there's a bug in `tidploy`.

This will simply say "Success!" (when run from `examples/run` directory).

```
tidploy run -c -x example_echo.sh
```
```
> Running example_echo.sh!
> Success!
```

As you can see, we are using `-c`. This ensures we really are running the file in our current directory. By default, `tidploy` will look for the nearest parent Git repository and take its root directory as the relative directory to run the given entrypoint at. If you are currently in this repository, then running the following is equivalent:

```
tidploy run -x examples/run/example_echo.sh
```

As you can see in the examples `example_spinner.sh`, `example_input.sh` and `example_stderr.sh`, `tidploy` will do what you expect when running programs that, respectively, replace a character multiple times using backspace, use stdin and have interleaved stderr and stdout printing.

### Using a secret

Now let's do something useful! First, let's set a secret using the following command:

```
tidploy secret some_key
```

It will prompt you to then enter a secret value:

```
> Enter secret:

> Set secret with store key tidploy::tidploy_root::<hash>:some_key!
```

Now that it's saved, we can run our example:

```
tidploy run -x examples/run/example_secret.sh -v some_key TIDPLOY_SOME_SECRET
```
```
> Running examples/run/example_secret.sh!
> <the password you set>
```

The `-v` option takes a list of arguments (of the form `K1 V1 K2 V2 ...`), where for each `Ki Vi` it will set the environment variable `Vi` for your executable to the secret you saved for key `Ki`.