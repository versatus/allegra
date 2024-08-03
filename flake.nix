{
  description = "Versatus rust-based project template.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    versatus-nix = {
      url = "github:versatus/versatus.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, versatus-nix, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          inherit (pkgs) lib;

          versaLib = versatus-nix.lib.${system};
          rustToolchain = versaLib.toolchains.mkRustToolchainFromTOML
            ./rust-toolchain.toml
            "sha256-6eN/GKzjVSjEhGO9FhWObkRFaE1Jf+uqMSdQnb8lcB4=";

          # Overrides the default crane rust-toolchain with fenix.
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain.fenix-pkgs;
          # Overrides the default crane filter to include protobuf, bash and service files.
          src =
            let
              filterProtoSources = path: _type: builtins.match ".*proto$" path != null;
              filterBinSources = path: _type: builtins.match ".*sh$" path != null;
              filterServiceSources = path: _type: builtins.match ".*service$" path != null;
              filter = path: type:
                (filterBinSources path type)
                || (filterServiceSources path type)
                || (filterProtoSources path type)
                || (craneLib.filterCargoSources path type);
            in
            lib.cleanSourceWith {
              inherit filter;
              src = ./.;
              name = "source";
            };

          # Common arguments can be set here to avoid repeating them later
          commonArgs = {
            inherit src;
            strictDeps = true;

            # Inputs that must be available at the time of the build
            nativeBuildInputs = with pkgs; [
              pkg-config # necessary for linking OpenSSL
              cmake
              protobuf
            ];

            buildInputs = with pkgs; [
              openssl.dev
              libvirt
            ] ++ [
              rustToolchain.darwin-pkgs
            ] ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.CoreServices
              pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            ];

            # Additional environment variables can be set directly
            # LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            # MY_CUSTOM_VAR = "some value";
          };

          # Build *just* the cargo dependencies, so we can reuse
          # all of that work (e.g. via cachix) when running in CI
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          # Build the actual crate itself, reusing the dependency
          # artifacts from above.
          mkCrateDrv = pname:
            craneLib.buildPackage (commonArgs // {
              inherit cargoArtifacts pname;
              doCheck = false;
              cargoExtraArgs = "--locked --bin ${pname}";
            });

          server = mkCrateDrv "server";
          cli = mkCrateDrv "cli";
          vmm = mkCrateDrv "vmm";
          monitor = mkCrateDrv "monitor";
          broker = mkCrateDrv "broker";
          state = mkCrateDrv "state";
          quorum = mkCrateDrv "quorum";
          network = mkCrateDrv "network";
        in
        {
          checks = {
            # Build the crate as part of `nix flake check` for convenience
            inherit server cli vmm monitor broker state quorum network;

            # Run clippy (and deny all warnings) on the crate source,
            # again, reusing the dependency artifacts from above.
            #
            # Note that this is done as a separate derivation so that
            # we can block the CI if there are issues here, but not
            # prevent downstream consumers from building our crate by itself.
            # TODO: Uncomment once clippy checks pass
            # my-crate-clippy = craneLib.cargoClippy (commonArgs // {
            #   inherit cargoArtifacts;
            #   cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            # });

            my-crate-doc = craneLib.cargoDoc (commonArgs // {
              inherit cargoArtifacts;
            });

            # Check formatting
            my-crate-fmt = craneLib.cargoFmt {
              inherit src;
            };

            # Audit dependencies
            my-crate-audit = craneLib.cargoAudit {
              inherit src advisory-db;
            };

            # Audit licenses
            my-crate-deny = craneLib.cargoDeny {
              inherit src;
            };

            # Run tests with cargo-nextest
            # Consider setting `doCheck = false` on `my-crate` if you do not want
            # the tests to run twice
            my-crate-nextest = craneLib.cargoNextest (commonArgs // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
          };

          packages = (
            let
              pkgs = import nixpkgs {
                overlays = [
                  self.overlays.rust
                  self.overlays.allegra
                ];
                inherit system;
              };
            in
            {
              inherit server vmm monitor broker state quorum network;

              inherit (pkgs) cli;

            }
          ) // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
            my-crate-llvm-coverage = craneLib.cargoLlvmCov (commonArgs // {
              inherit cargoArtifacts;
            });
          };

          apps = {
            server = flake-utils.lib.mkApp {
              drv = server;
            };
            cli = flake-utils.lib.mkApp {
              drv = cli;
            };
            vmm = flake-utils.lib.mkApp {
              drv = vmm;
            };
            monitor = flake-utils.lib.mkApp {
              drv = monitor;
            };
            broker = flake-utils.lib.mkApp {
              drv = broker;
            };
            state = flake-utils.lib.mkApp {
              drv = state;
            };
            quorum = flake-utils.lib.mkApp {
              drv = quorum;
            };
            network = flake-utils.lib.mkApp {
              drv = network;
            };
          };

          devShells.default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self.checks.${system};

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            # Extra inputs can be added here; cargo and rustc are provided by default.
            #
            # In addition, these packages and the `rustToolchain` are inherited from checks above:
            # cargo-audit
            # cargo-deny
            # cargo-nextest
            packages = with pkgs; [
              # ripgrep
              nil # nix lsp
              nixpkgs-fmt # nix formatter
            ];
          };

          formatter = pkgs.nixpkgs-fmt;
        }) // {
      overlays = {
        rust = final: prev: {
          rustToolchain = versatus-nix.lib.${final.system}.toolchains.mkRustToolchainFromTOML
            ./rust-toolchain.toml
            "sha256-6eN/GKzjVSjEhGO9FhWObkRFaE1Jf+uqMSdQnb8lcB4=";

          craneLib = (crane.mkLib final).overrideToolchain final.rustToolchain.fenix-pkgs;
        };
        allegra = import ./overlay.nix;
      };
    };
}
