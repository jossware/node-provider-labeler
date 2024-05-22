# node-provider-labeler

## Testing with [kwok](https://kwok.sigs.k8s.io/)

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
