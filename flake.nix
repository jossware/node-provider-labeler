{
  description = "labels (or annotates) Kubernetes Nodes with information from `.spec.ProviderID`";

  inputs = {
    nixpkgs.url = "nixpkgs"; # Resolves to github:NixOS/nixpkgs
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        kwokctl = pkgs.buildGoModule rec {
          pname = "kwokctl";
          version = "0.5.1";

          src = pkgs.fetchFromGitHub {
            rev = "v${version}";
            owner = "kubernetes-sigs";
            repo = "kwok";
            sha256 = "sha256-BTlg9zg3S1fwG6Gb4PYAcnlgPNC8sGkP1K9wYmuOPmU=";
          };

          vendorHash = "sha256-Wr7MZ2LLxKE84wmItEnJj8LhxMca4rooadiv4ubx/38=";

          nativeBuildInputs = [ pkgs.installShellFiles ];

          subPackages = [ "cmd/kwokctl" ];

          CGO_ENABLED = 0;

          ldflags = [
            "-s"
            "-w"
          ];

          doCheck = false;

          postInstall = ''
            installShellCompletion --cmd kwokctl \
              --bash <($out/bin/kwokctl completion bash) \
              --fish <($out/bin/kwokctl completion fish) \
              --zsh <($out/bin/kwokctl completion zsh)
          '';
        };

        create-cluster = pkgs.writeScriptBin "create-cluster" ''
          #!/usr/bin/env bash
          set -euo pipefail

          kwokctl create cluster --name kwok --runtime podman
        '';

        create-nodes = pkgs.writeScriptBin "create-nodes" ''
          #!/usr/bin/env bash
          set -euo pipefail

          COUNT=''${1:-3}

          for ((i=1; i<=COUNT; i++)); do
            kubectl apply -f - <<EOF
          apiVersion: v1
          kind: Node
          metadata:
            annotations:
              node.alpha.kubernetes.io/ttl: "0"
              kwok.x-k8s.io/node: fake
            labels:
              type: kwok
            name: kwok-node-$i
          spec:
            providerID: krok://kwokctl/kwok-node-$i
            taints: # Avoid scheduling actual running pods to fake Node
            - effect: NoSchedule
              key: kwok.x-k8s.io/node
              value: fake
          EOF
          done
        '';

        delete-cluster = pkgs.writeScriptBin "delete-cluster" ''
          #!/usr/bin/env bash
          set -euo pipefail
          kwokctl delete cluster --name kwok
        '';

        delete-nodes = pkgs.writeScriptBin "delete-nodes" ''
          #!/usr/bin/env bash
          set -euo pipefail
          kubectl delete node --all
        '';

        remove-label = pkgs.writeScriptBin "remove-label" ''
          #!/usr/bin/env bash
          set -euo pipefail
          kubectl label node ''$1 ''$2-
        '';

        remove-annotation = pkgs.writeScriptBin "remove-annotation" ''
          #!/usr/bin/env bash
          set -euo pipefail
          kubectl annotate node ''$1 ''$2-
        '';
      in
      with pkgs;
      {
        devShell = pkgs.mkShell {
          nativeBuildInputs =
            [ ]
            ++ lib.optionals (pkgs.stdenv.isDarwin) [
              libiconv
              darwin.apple_sdk.frameworks.SystemConfiguration
            ];
          buildInputs = with pkgs; [
            kubectl
            kwokctl
            create-cluster
            create-nodes
            delete-nodes
            delete-cluster
            remove-label
            remove-annotation
            pest-ide-tools
            rust-bin.stable.latest.default
            rust-analyzer
          ];
        };
      }
    );
}
