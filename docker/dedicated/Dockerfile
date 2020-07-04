FROM debian:buster-slim

ARG STEWARD_VERSION
ARG CONFIG_URL=https://github.com/timwie/steward/releases/download/${STEWARD_VERSION}/config.tar.gz

ARG DEDICATED_URL=http://files.v04.maniaplanet.com/server/TrackmaniaServer_2020-07-02.zip

RUN apt-get update \
    && apt-get -y install unzip wget

RUN groupadd -r trackmania -g 1000 \
    && useradd -u 1000 -g 1000 -m trackmania

WORKDIR /home/trackmania/

COPY updater.sh updater.sh
RUN chmod +x ./updater.sh

USER trackmania

RUN wget $DEDICATED_URL -qO /tmp/dedicated.zip

RUN unzip -q /tmp/dedicated.zip -d ./ \
    && rm /tmp/dedicated.zip \
    && rm -rf ./*.bat ./*.exe ./*.html ./RemoteControlExamples \ 
    && chmod +x ./TrackmaniaServer

RUN wget $CONFIG_URL -qO /tmp/config.tar.gz

RUN tar -xf /tmp/config.tar.gz \
    && rm /tmp/config.tar.gz \
    && rm steward.toml

EXPOSE 2350 2350/udp 3450 3450/udp 5000

CMD [ "./TrackmaniaServer", \
      "/nodaemon",           \
      "/dedicated_cfg=dedicated_cfg.txt",        \
      "/game_settings=MatchSettings/maplist.txt" \
]
