# Fractal Explorer

Fractal Explorer is a small web app for exploring the Mandelbrot set, Julia
sets, the Burning Ship, and the Tricorn. It combines a Rust image-tile server
with a Leaflet-based browser interface: pan and zoom around the complex plane,
adjust the colors and rendering detail, and share a view with its URL.

## How it works

The project has two main parts:

- `src/main.rs` is an Axum HTTP server and fractal renderer. It maps each
  pixel in a 256 × 256 tile to a point in the complex plane, calculates a
  smooth escape-time value, applies a color palette, and encodes the result as
  a PNG. CPU-heavy rendering runs on Tokio's blocking thread pool.
- `static/index.html` contains the interface and Leaflet map. Leaflet requests
  tiles from `/tile/{z}/{x}/{y}` as the view changes. Fractal type, palette,
  color-band density, detail, and (for Julia sets) the Julia constant are
  passed as query parameters.

At zoom level 0, a tile covers a 4 × 4 area of the complex plane; each zoom
level halves the area. The server and browser support zoom levels up to 46,
where the precision of 64-bit floating-point coordinates becomes a limit.
Right-clicking or long-pressing the Mandelbrot view opens the Julia set for
that point. The URL hash preserves the current view and settings for sharing.

The HTML is embedded in the Rust executable at compile time. Leaflet and the
Figtree font are loaded from public CDNs by the browser, so those assets need
to be reachable by visitors.

## Run with Docker Compose

Prerequisites: Docker Engine and the Docker Compose plugin.

From the repository directory, build and start the app:

```bash
docker compose up -d --build
```

The first build may take a few minutes because the release image compiles the
Rust application with optimizations. Compose binds the app to
`127.0.0.1:8085` on the host, forwarding to port `8080` in the container. Check
that it is serving the page:

```bash
curl -I http://127.0.0.1:8085/
```

Open <http://127.0.0.1:8085/> in a browser on the host. To stop the app:

```bash
docker compose down
```

## Run with Rust

Prerequisites: a Rust toolchain with Cargo. From the repository directory:

```bash
cargo run --release
```

The server listens on `0.0.0.0:8080` by default. Set `PORT` to choose another
port, for example:

```bash
PORT=8085 cargo run --release
```

Then open <http://127.0.0.1:8080/> (or the port you configured). The release
profile enables optimization and link-time optimization.

## Configuration and controls

The server accepts `PORT` as an environment variable. No other server
configuration is required. In the web interface:

- Drag to pan and scroll or pinch to zoom.
- Choose Mandelbrot, Julia, Burning Ship, or Tricorn.
- Select one of five palettes and adjust color-band density and detail.
- Change the Julia constant directly, or right-click/long-press a point in
  the Mandelbrot set to use that point as the constant.
- Use **Reset view** to return to the default extent or **Copy link** to share
  the current fractal, position, zoom, and settings.

## Optional: serve publicly with nginx

For a public deployment, the Compose configuration exposes the app only on
the host's loopback interface. Place nginx in front of it, update the example
domain below, and enable the site:

```nginx
# Cache rendered tiles on disk so popular areas are instant
proxy_cache_path /var/cache/nginx/fractal levels=1:2 keys_zone=fractal:10m
                 max_size=2g inactive=30d use_temp_path=off;

server {
    server_name fractal.example.com;

    location / {
        proxy_pass http://127.0.0.1:8085;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /tile/ {
        proxy_pass http://127.0.0.1:8085;
        proxy_cache fractal;
        proxy_cache_valid 200 30d;
        add_header X-Cache $upstream_cache_status;
    }
}
```

Save this as `/etc/nginx/sites-available/fractal`, then enable it and
optionally obtain a TLS certificate with Certbot:

```bash
sudo ln -s /etc/nginx/sites-available/fractal /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d fractal.example.com
```

Replace `fractal.example.com` with your domain. Ensure DNS points to the
server and that nginx and the firewall allow public HTTP/HTTPS traffic.
