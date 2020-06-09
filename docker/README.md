# Deploying with `docker-compose`
Setting up a dedicated server with Steward can be automated using `docker-compose`,
where server, database and controller will live in seperate containers, within the
same network:

- `$ git clone https://github.com/timwie/steward.git`
- `$ cd steward/docker`
- Edit the `.env` file accordingly. These variables will be used in the build process.
  - `STEWARD_VERSION` is a version tag like `v0.1.0`.
  - `DEDICATED_URL` lets you choose a specific dedicated server version.
    Use `http://files.v04.maniaplanet.com/server/ManiaplanetServer_Latest.zip` if you don't
    care about a specific version.
  - `TITLE` is a title name like "TMStadium@nadeo".
  - `TITLE_PACK_URL` is the download location of the `xyz.Title.Pack.gbx` file. 
- Create the following docker volumes:
  - `$ docker volume create --name steward-controller`
  - `$ docker volume create --name steward-dedicated`
  - `$ docker volume create --name steward-postgres`
- Run `docker-compose up -d` to build & start the services in containers.
- All services are configured to be restarted whenever they go down.

<br>

## Useful Commands
- All data is accessible in the volumes, independent of the containers.
  You can use something like `docker run -it --rm --volume <volume>:/mnt <image> bash`
  to open a container in which you can edit configuration files, or make backups. In this case,
  the data would be in the `/mnt` directory.

- You can directly inspect the database using `$ docker run -it --rm --network container_default postgres psql -h database -U steward`.

- The service logs can be viewed with `$ docker logs -f <container id>`.

- You can update the server by using `$ docker-compose run dedicated bash`, and
`$ ./updater.sh`. Run `$ docker-compose restart` afterwards.

- You can update the controller by using `$ docker-compose run -u 0 controller bash`, and
  `$ ./updater.sh`. Run `$ docker-compose restart` afterwards. Refer to the changelog
  to see if you need to update your config.

<br>

## Troubleshooting
- When editing the server config in `dedicated_cfg.txt`, make sure that to allow remote XML-RPC access
  with `<xmlrpc_allowremote>True</xmlrpc_allowremote>`, otherwise the server will not allow the controller to connect.
