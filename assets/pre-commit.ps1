#!/usr/bin/env pwsh

function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

cargo check
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
ThrowOnNativeFailure

cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
ThrowOnNativeFailure

cargo doc
ThrowOnNativeFailure

# copy the tree to the WSL file system to improve compile times
wsl rsync --delete -av /mnt/c/Users/fenhl/git/github.com/fenhl/wheel/stage/ /home/fenhl/wslgit/github.com/fenhl/wheel/ --exclude target
ThrowOnNativeFailure

wsl env -C /home/fenhl/wslgit/github.com/fenhl/wheel cargo check
ThrowOnNativeFailure

wsl env -C /home/fenhl/wslgit/github.com/fenhl/wheel cargo check --manifest-path=crate/wheel/Cargo.toml --no-default-features
ThrowOnNativeFailure

wsl env -C /home/fenhl/wslgit/github.com/fenhl/wheel cargo check --manifest-path=crate/wheel/Cargo.toml --all-features
ThrowOnNativeFailure
