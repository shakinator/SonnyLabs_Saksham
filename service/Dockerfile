FROM docker.io/rust:1.76.0-bookworm as build

RUN apt-get update && apt-get install -y libpython3.11-dev

WORKDIR /src

COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY src/ src/
COPY .sqlx/ .sqlx/

RUN mkdir /app
RUN --mount=type=cache,target=/src/target \
    cargo build --release --bin server \
    && cp /src/target/release/server /app/server

RUN --mount=type=cache,target=/src/target \
    cargo build --release --bin db-seed \
    && cp /src/target/release/db-seed /app/db-seed


FROM build as test

WORKDIR /src

RUN apt-get update && apt-get install -y \
    libpython3.11 python3-pip python3-pip-whl python3-venv \
    libfontconfig1 \
    tini

RUN python3 -m venv _venv
ENV PATH=/src/_venv/bin:$PATH
ENV PYTHONPATH=/src/_venv/lib/python3.11/site-packages/

COPY requirements.txt requirements.txt
RUN pip install -r requirements.txt

COPY pickles/ pickles/

RUN --mount=type=cache,target=/src/target \
    cargo test --no-run \
    && cp -p -r /src/target /src/_target
RUN mv /src/_target /src/target

ENTRYPOINT ["tini", "--", "cargo", "test"]


FROM docker.io/debian:bookworm-slim as python-build

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libpython3.11 python3-pip python3-pip-whl python3-venv

RUN python3 -m venv _venv
ENV PATH=/app/_venv/bin:$PATH
ENV PYTHONPATH=/app/_venv/lib/python3.11/site-packages/

COPY requirements.txt requirements.txt
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt


FROM docker.io/debian:bookworm-slim as service

RUN apt-get update && apt-get install -y \
    libpython3.11 \
    libfontconfig1 \
    tini

WORKDIR /app

COPY --from=python-build /app/_venv _venv
ENV PATH=/app/_venv/bin:$PATH
ENV PYTHONPATH=/app/_venv/lib/python3.11/site-packages/

COPY --from=build /app/server server
COPY --from=build /app/db-seed db-seed
COPY pickles/ pickles/

COPY entrypoint.sh /app/entrypoint.sh

ENV DATABASE_URL="<UNSET>"
EXPOSE 3000

ENTRYPOINT ["tini", "--", "/app/entrypoint.sh"]
