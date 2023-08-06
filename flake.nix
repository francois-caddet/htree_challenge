{
  description = "Root package of a client/server file storage backed by a hash tree.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;
        rust = pkgs.rust-bin.nightly.latest.default;

        craneLib = crane.lib.${system}.overrideToolchain rust;

        src = craneLib.cleanCargoSource (craneLib.path ./.);

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;

          buildInputs = with pkgs; [
            # Add additional build inputs here
            pkg-config
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        htree-challenge = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
        htree-server = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--bin htree-server";
        });
        htree-client = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--bin htree-client";
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit htree-challenge;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          htree-challenge-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          htree-challenge-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          htree-challenge-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          htree-challenge-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `htree-challenge` if you do not want
          # the tests to run twice
          htree-challenge-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        } // lib.optionalAttrs (system == "x86_64-linux") {
          # NB: cargo-tarpaulin only supports x86_64 systems
          # Check code coverage (note: this will not upload coverage anywhere)
          htree-challenge-coverage = craneLib.cargoTarpaulin (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        packages = {
          inherit htree-challenge;
          inherit htree-server;
          inherit htree-client;
          default = htree-challenge;
        } // lib.optionals pkgs.stdenv.isLinux {
          server-docker = pkgs.dockerTools.buildImage {
            name = "server-docker";
            config = {
              Expose = [2636];
              EntryPoint = [ "${htree-server}/bin/htree-server" "0.0.0.0"];
            };
          };
          client-docker = pkgs.dockerTools.buildImage {
            name = "client-docker";
            config = {
              EntryPoint = [ "${htree-client}/bin/htree-client"];
            };
          };
        };

        apps = rec {
          client = flake-utils.lib.mkApp {
            drv = htree-client;
            name = "htree-client";
          };
          server = flake-utils.lib.mkApp {
            drv = htree-server;
            name = "htree-server";
          };
          default = client;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs
          buildInputs = [
          ];

          # Extra inputs can be added here
          nativeBuildInputs = with pkgs; [
            cargo-edit
            rust-analyzer
            rustfmt
            nixpkgs-fmt
            pkg-config
          ];
        };
      });
}
