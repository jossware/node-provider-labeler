# Contributing

All contributions are welcome.

[Issues](https://github.com/jossware/node-provider-labeler/issues)
[Pull Requests](https://github.com/jossware/node-provider-labeler/pulls)

## Testing

### Testing with [kwok](https://kwok.sigs.k8s.io/)

You can use interactive commands to test the controller locally using kwok.

*Note: required tooling configured via the [nix](https://nixos.org/) [flake](./flake.nix).*

1. Create cluster
    ``` shell
    create-cluster
    ```
1. Create Nodes

    ``` shell
    create-nodes 25
    ```
1. Run controller

Rinse and repeat as necessary.

Cleanup with `delete-nodes` and `delete-cluster`.

## Releasing

`release.sh` performs much of the tedium that comes with a new release. Use it
to prep a new version of the controller, a new version of the Helm chart, or
both.

```shell
# release just the app
./scripts/release.sh app                                                                                                                │
# release just the chart
./scripts/release.sh chart
# release the app and the chart
./scripts/release.sh both                                                                                                               │
```

After running, review the diff, commit, create a pull request, and merge once
approved. At this point, you can create a tag for the controller or chart
release. For example:

```shell
# chart tag
git tag chart-0.18.0 --message "chart 0.18.0"

# controller tag
git tag v0.8.0 --message "node-provider-labeler 0.8.0"

# push tag
git push origin --tags
```

## License

All code in this repository is under the or the [MIT] license.

[MIT]: https://opensource.org/licenses/MIT
