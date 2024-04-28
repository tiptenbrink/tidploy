### Example echo

The most simple way to use `tidploy` is to just run some other executable. While this is not something that actually uses any features from `tidploy`, it should still just work and any issues with it mean there's a bug in `tidploy`.

This will simply say "Success!"

```
tidploy run -x example_echo.sh --context none
```
```
> Running example_echo.sh!
> Success!
```

As you can see, we are using `--context none`. This ensures we really are running the file in our current directory. By default, `tidploy` will look for the nearest parent Git repository and take its root directory as the relative directory to run the given entrypoint at. If you are currently in this repository, then running the following is equivalent:

```
tidploy run -x examples/run/example_echo.sh
```

### Using a secret