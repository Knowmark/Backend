FROM rust:1 as builder

WORKDIR /home/knowmark_backend

COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/bin/server.rs
RUN cargo build --release --bin knowmark-server

COPY . .
# Lets docker know main.rs was updated by COPY
RUN touch src/bin/server.rs
RUN cargo test
RUN cargo build --release --bin knowmark-server

RUN strip target/release/knowmark-server

FROM scratch
WORKDIR ${WORK_DIR}
COPY --from=builder /home/knowmark_backend/target/x86_65-unknown-linux-gnu/release/knowmark-server /usr/local/bin/knowmark-server
COPY --from=builder /home/knowmark_backend/Knowmark.toml ${CONFIG_DIR}/Knowmark.toml

EXPOSE 8000
ENTRYPOINT ["knowmark-server"]
