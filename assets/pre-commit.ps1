#!/usr/bin/env pwsh

function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

cargo check
ThrowOnNativeFailure

cargo msrv --manifest-path=crate/wheel/Cargo.toml verify
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=github
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features --features=racetime
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
ThrowOnNativeFailure

cargo doc
ThrowOnNativeFailure

wsl -d ubuntu-m2 sudo -n apt-get -y install libglib2.0-dev
ThrowOnNativeFailure

# copy the tree to the WSL file system to improve compile times
wsl -d ubuntu-m2 rsync --mkpath --delete -av /mnt/c/Users/fenhl/git/github.com/fenhl/wheel/stage/ /home/fenhl/wslgit/github.com/fenhl/wheel/ --exclude target
ThrowOnNativeFailure

wsl -d ubuntu-m2 env -C /home/fenhl/wslgit/github.com/fenhl/wheel /home/fenhl/.cargo/bin/cargo check
ThrowOnNativeFailure

wsl -d ubuntu-m2 env -C /home/fenhl/wslgit/github.com/fenhl/wheel /home/fenhl/.cargo/bin/cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
ThrowOnNativeFailure

wsl -d ubuntu-m2 env -C /home/fenhl/wslgit/github.com/fenhl/wheel /home/fenhl/.cargo/bin/cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
ThrowOnNativeFailure
