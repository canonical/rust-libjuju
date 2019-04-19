rust-libjuju
============

This repository hosts a library called `libjuju` for interacting with
Juju from within Rust, as well as several Rust-based Juju plugins


Setup
-----

Soon you will be able to snap install the utilities from this repo:

    sudo snap install juju-kubectl --classic --edge
    sudo snap install juju-bundle --classic --edge

Until then, or if you want to run these utilities from source, clone this repo
with

    https://github.com/knkski/rust-libjuju.git
    cd rust-libjuju

You will also need to install the Rust compiler. Instructions can be found at
https://rustup.rs/.


juju-kubectl
------------

This plugin will run `kubectl` on the specified controller.

You can run this plugin and just pass in the appropriate `kubectl`
arguments:

    # Installed via snap
    juju kubectl get nodes

    # Running from source
    cargo run --bin juju-kubectl get nodes

Note that both `cargo` and this plugin can take arguments, so to pass
options or parameters to `kubectl` itself, you will want to call it
like so:

    # Installed via snap
    juju kubectl -c controller-name -- get nodes

    # Running from source
    cargo run --bin juju-kubectl -- -c controller-name -- get nodes


juju-bundle
-----------

This plugin will build any charms as necessary in a bundle, and then
deploy it with `juju deploy`.

You can run this plugin and just pass in the appropriate juju `deploy`
commands:

    # Installed via snap
    juju bundle deploy bundle.yaml

    # Running from source
    cargo run --bin juju-bundle deploy bundle.yaml

Note that both `cargo` and this plugin can take arguments, so to pass
options or parameters to `juju deploy` itself, you will want to call it like
so:

    # Installed via snap
    juju bundle deploy bundle.yaml -- -m model-name

    # Running from source
    cargo run --bin juju-bundle -- deploy bundle.yaml -- -m model-name
