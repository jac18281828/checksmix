FROM ghcr.io/jac18281828/rustdev:latest

ARG PROJECT=rusthello
WORKDIR /workspaces/${PROJECT}
COPY --chown=jac:jac . .
ENV USER=jac
USER jac

ENV PATH=/home/jac/.cargo/bin:$PATH
# source $HOME/.cargo/env
RUN rustc --version

RUN cargo fmt --check
RUN cargo clippy --all-features --no-deps
RUN cargo test
CMD cargo run
