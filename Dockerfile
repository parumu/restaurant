FROM rustlang/rust:nightly

COPY . .
RUN cargo install --path application
CMD ["application"]
