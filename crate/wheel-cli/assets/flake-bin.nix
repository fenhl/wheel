{
    inputs.flake.url = "github:fenhl/flake";
    outputs = attrs: attrs.flake.lib {
        packages.default = { pkgs, lib, ... }: let
            manifest = (lib.importTOML ./Cargo.toml).package;
        in pkgs.rustPlatform.buildRustPackage {
            inherit (manifest) version;
            pname = manifest.name;
            src = ./.;
            cargoLock = {
                allowBuiltinFetchGit = true; # allows omitting cargoLock.outputHashes
                lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
                installShellFiles # required for `installShellCompletion` in postInstall hook
            ];
            postInstall = let
                bin = "${pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages} $out/bin/${manifest.name}";
            in lib.optionalString (pkgs.stdenv.hostPlatform.emulatorAvailable pkgs.buildPackages) ''
                installShellCompletion --cmd ${manifest.name} \
                    --bash <(COMPLETE=bash ${bin}) \
                    --fish <(COMPLETE=fish ${bin}) \
                    --zsh <(
                        COMPLETE=zsh ${bin}
                        # fix completion not working on first Tab press of shell session
                        echo '_clap_dynamic_completer_${manifest.name}'
                    )
            '';
        };
    };
}
