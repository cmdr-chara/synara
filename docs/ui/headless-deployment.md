# Deploy the headless workspace over HTTPS

The supported deployment is a Linux service behind a TLS reverse proxy on the
same host. Synara still binds only loopback. `--public-origin` enables one exact
HTTPS Host/Origin, and forwarded headers never authorize a request. The bearer
token is required for every workspace API. This is a single-user service with
that user's filesystem and provider permissions.

## Build and install

Build the pinned Rust workspace on Linux, then create the deployment bundle:

```sh
cargo +1.98.1 build --locked --release -p synara-server
python3 scripts/package_server.py --binary target/release/synara-server \
  --version 0.1.0-YOUR_COMMIT --target x86_64-unknown-linux-gnu --output-dir dist
```

The deterministic archive includes the binary, per-file SHA-256 manifest,
systemd unit, Caddy configuration and activation helper. Packages are unsigned.
Use a reviewed build or authenticated distribution channel. Checksums detect
changed bytes and are not proof of publisher identity. Trusted signed release
distribution remains the separate M28/A08 roadmap scope.

Extract a reviewed archive into a new directory and run:

```sh
sudo python3 deploy/manage.py install
```

This stages an immutable version under `/opt/synara-server/releases`, atomically
switches `current`, and retains `previous`. It never changes the database,
credentials, service configuration or an existing release directory. Concurrent
activations are refused. The helper verifies file hashes and host architecture.

Create a dedicated `synara` service user with home `/var/lib/synara`, a private
`/etc/synara` configuration directory, and a `/srv/synara` workspace owned by that
user. Put a cryptographically random token of at least 32 printable characters
in `/etc/synara/token`, owned by `synara` with mode `0600`. Keep the token out of
URLs, command history and proxy access logs.

Copy `deploy/server.env` to `/etc/synara/server.env`, replacing the example HTTPS
origin. Copy `deploy/synara-server.service` to `/etc/systemd/system/`. This service
allows writes only to its private state/runtime directories and `/srv/synara`.
Install required provider CLIs for the service user and register projects under
that workspace. Add any additional reviewed writable paths explicitly to the
unit. Saved automations stay disarmed unless the operator adds `--automations`.

## TLS and startup

Use the included `deploy/Caddyfile` with the same DNS host as `server.env`.
Point DNS at the host and make ports 80/443 available for Caddy. Do not publish
backend port 17341. Caddy manages HTTPS certificates and preserves the original
Host while proxying to loopback. Do not replace the client's Origin header.

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now synara-server
sudo systemctl reload caddy
curl --fail https://YOUR_HOST/ready
```

`/ready` returns 503 until database ownership and recovery finish. Open the HTTPS
origin and enter the private bearer token. Wrong Host or Origin receives 421.
SIGINT and SIGTERM both stop owned runs and release database ownership.

For local use, omit `--public-origin` and retain the existing strict loopback
Host/Origin policy. No wildcard, remote plaintext bind, CORS wildcard or trusted
forwarded-header mode is added.

## Update and rollback

Stop the service and make a consistent backup of its SQLite database plus any
associated files before upgrading. Activate the next reviewed extracted bundle
with `deploy/manage.py install`, then restart the service and check `/ready`.
Configuration and provider credentials survive unchanged. If activation fails,
the currently selected release is retained. Keep old bundles until verification
completes.

If the previous binary supports the current database schema, stop the service
and run:

```sh
sudo python3 deploy/manage.py rollback --confirm-compatible-data
sudo systemctl restart synara-server
curl --fail https://YOUR_HOST/ready
```

For an incompatible schema, restore the reviewed backup before restarting the
previous binary. The helper switches binaries only and does not guess how to
reverse database migrations. This is an operator-managed deployment lifecycle,
not the desktop automatic updater.

References: [Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
and [automatic HTTPS](https://caddyserver.com/docs/automatic-https).
