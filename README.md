# Fractal explorer

A Rust (Axum) tile server that renders the Mandelbrot set, Julia sets,
Burning Ship and Tricorn, with a Leaflet-based pan-and-zoom web interface.

## Run it

```bash
docker compose up -d --build
```

The first build takes a few minutes (Rust compiles with full optimisation).
Then test locally on the VPS:

```bash
curl -I http://127.0.0.1:8085/
```

## nginx

Add a server block, e.g. `/etc/nginx/sites-available/fractal`
(change the domain to yours):

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

Then enable it and get a certificate:

```bash
sudo ln -s /etc/nginx/sites-available/fractal /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d fractal.example.com
```

## Using it

- Scroll or pinch to zoom, drag to pan.
- Right-click on the Mandelbrot set to open the Julia set for that point.
- "Copy link" saves your exact position, fractal and colours in the URL.
- "Detail" raises the iteration count; turn it up if deep zooms look blobby.

Zoom goes to about 2^46 (around 70 trillion ×). Beyond that, 64-bit floats
run out of precision and the image turns pixelated.
