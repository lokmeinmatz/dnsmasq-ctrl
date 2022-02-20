FROM node:latest as frontend

WORKDIR /srv/frontend

COPY ./frontend/package.json .
COPY ./frontend/package-lock.json .
RUN npm i
# should ignore node_modules because of .dockerignore
COPY ./frontend .
RUN npm run build


FROM rust:1.58.1 as builder

WORKDIR /srv
RUN USER=root cargo new --bin dnsmasq-ctrl 
WORKDIR /srv/dnsmasq-ctrl
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/dnsmasq_ctrl*
RUN cargo build --release


FROM debian:stretch-slim
ARG APP=/srv/dnsmasq-ctrl

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata dnsmasq \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 80
EXPOSE 53

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

WORKDIR ${APP}
COPY --from=builder ${APP}/target/release/dnsmasq-ctrl ${APP}/dnsmasq-ctrl
RUN mkdir frontend
COPY --from=frontend /srv/frontend/dist ${APP}/frontend/dist
RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER

CMD ["./dnsmasq-ctrl"]