{
    inputs.flake.url = "github:fenhl/flake";
    outputs = attrs: attrs.flake.lib {
        devShells.default = { pkgs, ... }: {
            packages = with pkgs; [
                cargo
                glib
                pkg-config
                python3
            ];
        };
        packages.default = { pkgs, lib, ... }: pkgs.rustPlatform.buildRustPackage {
            inherit ((lib.importTOML ./Cargo.toml).workspace.package) version;
            name = "wheel";
            src = ./.;
            buildAndTestSubdir = "crate/wheel-cli";
            cargoLock = {
                allowBuiltinFetchGit = true; # allows omitting cargoLock.outputHashes
                lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
                installShellFiles # required for `installShellCompletion` in postInstall hook
            ];
            postInstall = let
                wheel = "${pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages} $out/bin/wheel";
            in pkgs.lib.optionalString (pkgs.stdenv.hostPlatform.emulatorAvailable pkgs.buildPackages) ''
                installShellCompletion --cmd wheel \
                    --bash <(COMPLETE=bash ${wheel}) \
                    --fish <(COMPLETE=fish ${wheel}) \
                    --zsh <(
                        COMPLETE=zsh ${wheel}
                        # fix completion not working on first Tab press of shell session
                        echo '_clap_dynamic_completer_wheel'
                    )
            '';
        };
    };
}
