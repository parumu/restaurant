FROM rustlang/rust:nightly

COPY . .
RUN cargo install --path .
CMD ["restaurant"]
