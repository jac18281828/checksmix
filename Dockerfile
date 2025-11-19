FROM ghcr.io/jac18281828/rust:latest

ARG PROJECT=checksmix
WORKDIR /workspaces/${PROJECT}
COPY --chown=rust:rust . .
ENV USER=rust
USER rust

ENV PATH=/home/${USER}/.cargo/bin:$PATH
# source $HOME/.cargo/env
