FROM debian:buster-slim

ARG STEWARD_VERSION
ARG CONFIG_URL=https://github.com/timwie/steward/releases/download/${STEWARD_VERSION}/config.tar.gz

ARG DEDICATED_URL=http://files.v04.maniaplanet.com/server/TrackmaniaServer_Latest.zip

RUN apt-get update \
    && apt-get -y install nano unzip wget

RUN groupadd -r trackmania -g 1000 \
    && useradd -u 1000 -g 1000 -m trackmania

WORKDIR /home/trackmania/

COPY updater.sh updater.sh
RUN chmod +x ./updater.sh

USER trackmania

RUN wget -q $DEDICATED_URL
RUN unzip -q TrackmaniaServer_Latest.zip \
    && rm TrackmaniaServer_Latest.zip \
    && rm -rf ./*.bat ./*.exe ./*.html ./RemoteControlExamples \ 
    && chmod +x ./TrackmaniaServer

RUN wget -q $CONFIG_URL
RUN tar -xf config.tar.gz && \
    rm config.tar.gz && \
    rm steward.toml

EXPOSE 2350 2350/udp 3450 3450/udp 5000

CMD [ "./TrackmaniaServer", \
      "/nodaemon",           \
      "/dedicated_cfg=dedicated_cfg.txt",        \
      "/game_settings=MatchSettings/timeattack.txt" \
]
