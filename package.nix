{ lib
, pkg-config
, cmake
, protobuf
, openssl
, libvirt
, stdenv
, darwin

, rustToolchain
, craneLib
, pname ? "cli"
}:

let
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


  commonArgs = {
    inherit src;
    strictDeps = true;

    # Inputs that must be available at the time of the build
    nativeBuildInputs = [
      pkg-config # necessary for linking OpenSSL
      cmake
      protobuf
    ];

    buildInputs = [
      openssl.dev
      libvirt
    ] ++ [
      rustToolchain.darwin-pkgs
    ] ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.CoreServices
      darwin.apple_sdk.frameworks.CoreFoundation
    ];

    # Additional environment variables can be set directly
    # LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    # MY_CUSTOM_VAR = "some value";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in

craneLib.buildPackage (commonArgs // {
  inherit cargoArtifacts pname;
  doCheck = false;
  cargoExtraArgs = "--locked --bin ${pname}";
})
