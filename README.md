# Basalt CLI

CLI tool for building docker images for Basalt programming competitions,
creating and verifying configurations, and much more.

## Installation

```sh
cargo install --git https://basalt-rs/basalt-cli
```

## Usage

Create a configuration with `basalt init`. Configure to your heart's desire
in consultation with
[the docs](https://basalt.rs/docs/configuration/configuring-basalt/).
Then build your container image with `basalt build`.

Learn more about the Basalt CLI in the
[docs](https://basalt.rs/docs/cli/build/).

## About the Container

The container is built upon [Fedora](https://hub.docker.com/_/fedora) for its
qualities of being fairly up-to-date while also being stable. The following
files and directories are worth noting:

| Path                           | Purpose                                                         |
| ------------------------------ | --------------------------------------------------------------- |
| `/usr/local/bin/basalt-server` | Basalt server binary                                            |
| `/opt/basalt/web/`             | Contains Basalt static web files are stored if this is enabled  |
| `/var/log/basalt/`             | Contains Basalt logs                                            |
| `/execution/`                  | Contains all competition runtime data (scripts, config, etc)    |

## Networking

Since your Basalt competition is ran inside of a container, loopback addresses
and other DNS information might be a bit different. You have three main ways
to deal with this:
- Docker `--add-host` DNS mapping
- Update `localhost` to be `host.docker.internal`
- Use the host network

You'll have to assess which of these strategies is best for your situation,
but here's an example of how each of these approaches can solve a realistic
problem.

Let's say we want to forward Basalt server events to a server accessible at our
host's loopback address at port 8081 with the configuration below:

```toml
[integrations]
webhooks = "http://localhost:8081/events"
```

We will also be building the competition image with the following command:

```sh
basalt build -t basalt-server-eventing
```

> [!NOTE]
> This example uses Docker, so you may have to adjust for different container
> backends.

### Using the `--add-host` Flag

In this example, we just map `eventing` to
[host-gateway](https://docs.docker.com/reference/cli/dockerd/#configure-host-gateway-ip)
to ensure Docker understands where `http://eventing:8081` is. It does require
you update the configuration.

```toml
[integrations]
webhooks = "http://eventing:8081/events"
```

```sh
docker run \
  -p 8080:9090 \
  --add-host=eventing:host-gateway \
  basalt-server-eventing
```

### Using `host.docker.internal`

Depending on your Docker configurations, this solution and the previous may
need to be used in tandem.

In this example, we simply use `host.docker.internal` instead of
`localhost`.

```toml
[integrations]
webhooks = "http://host.docker.internal:8081/events"
```

```sh
docker run \
  -p 8080:9090 \
  basalt-server-eventing
```

### Use Host Network

Using the host network with Docker is quite simple.

```sh
docker run \
  -p 8080:9090 \
  --network host \
  basalt-server-eventing
```

One advantage of this solution is you avoid needing to update the
configuration entirely. The disadvantage is that using a dedicated network
as it would ordinarily by default is a bit more secure, so we lose one of
the networking security layers built around Basalt's sandbox.

Of the three solutions, we recommend this one last.
