# HTree Challenge

This repo contains the code for a programing challenge.

It contains 3 things:

- A lib: Implementation of a append only merkel tree with generic type of data attached to it.
- A server: `htree-server` which can recieve files, returned them and generate a proof for them thanks to the merkel tree.
- A client: `htree-client` which can sent files to the server, get them and prove a already downloaded file.
  When it get a file, it prove it before saving.

## Build

To build this repo, the standart way is to use Nix.
Anyway, it's a full rust project so, it can be build with cargo.
Nether the less, the build with cargo is not supported. Also, to build the Docker images, the only way is with Nix.

```
nix build .\#client # build the client
nix build .\#server # build the server
nix build .\#client-docker # build the client-docker
nix build .\#server-docker # build the server-docker
```

The binaray will be in `./result` so, to load the docker image, after building it, do `docker load < result`

## Run

For the server/client usage, pass the flag `--help` to the following commands.

### With Nix

Run the client:

```
nix run .#client <SERVER> [PORT] <CMD> <CMD ARGS>
```

Run the server:

```
nix run .#server [BINDADDR] [PORT]
```

As usual with Nix flakes, you can run `nix shell` to open a shell with `htree-server` and `htree-client` in the PATH.
