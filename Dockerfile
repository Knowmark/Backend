FROM rust:1 as builder

WORKDIR /home/knowmark_backend

COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY . .
# Lets docker know main.rs was updated by COPY
RUN touch src/main.rs
RUN cargo test
RUN cargo build --release

RUN strip target/release/knowmark_backend

FROM scratch
WORKDIR ${WORK_DIR}
COPY --from=builder /home/knowmark_backend/target/x86_65-unknown-linux-gnu/release/knowmark_backend /usr/local/bin/knowmark_backend
COPY --from=builder /home/knowmark_backend/Rocket.toml ${CONFIG_DIR}/Rocket.toml

EXPOSE 8000
ENTRYPOINT ["knowmark_backend"]
