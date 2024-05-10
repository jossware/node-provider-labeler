{
  description =
    "labels (or annotates) Kubernetes Nodes with information from `.spec.ProviderID`";

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

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShell = pkgs.mkShell {
          nativeBuildInputs = [ ] ++ lib.optionals (pkgs.stdenv.isDarwin) [
            libiconv
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          buildInputs = with pkgs; [
            kubectl
            pest-ide-tools
            rust-bin.stable.latest.default
            rust-analyzer
          ];
        };
      });
}
