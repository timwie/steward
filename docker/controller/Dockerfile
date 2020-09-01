FROM liuchong/rustup:stable

ARG STEWARD_VERSION
ARG CONTROLLER_URL=https://github.com/timwie/steward/releases/download/${STEWARD_VERSION}/steward-x86_64-unknown-linux-gnu
ARG CONFIG_URL=https://github.com/timwie/steward/releases/download/${STEWARD_VERSION}/config.tar.gz

RUN apt-get update \
    && apt-get -y install git nano unzip wget

RUN groupadd -r steward -g 1000 && \
    useradd -u 1000 -g 1000 -m steward

WORKDIR /home/steward/

COPY updater.sh updater.sh
RUN chmod +x ./updater.sh

USER steward

COPY .env .env

RUN wget -q $CONTROLLER_URL
RUN chmod +x ./steward-x86_64-unknown-linux-gnu

RUN wget -q $CONFIG_URL
RUN tar -xf config.tar.gz && \
    rm config.tar.gz && \
    rm -rf UserData

RUN sed -i 's/rpc_address.*/rpc_address = "dedicated:5000"/g' steward.toml
RUN sed -i 's/postgres_connection.*/postgres_connection = "host=database user=steward dbname=steward password=steward"/g' steward.toml

CMD [ "./steward-x86_64-unknown-linux-gnu" ]
