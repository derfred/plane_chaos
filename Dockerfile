# syntax=docker/dockerfile:1.7

# Website for Plane Chaos: the landing page at / and the WASM build of the
# game at /play/.
#
#   docker build -t plane-chaos-web .
#   docker run --rm -p 8080:8080 plane-chaos-web

# ---- Build the game for the browser ----
# The wasm output is the same for every platform, so build it natively on the
# build host instead of under emulation.
FROM --platform=$BUILDPLATFORM rust:1.99-trixie AS game
ARG BUILDARCH
ARG BINARYEN_VERSION=133

RUN rustup target add wasm32-unknown-unknown

WORKDIR /src
COPY Cargo.toml Cargo.lock ./

# wasm-bindgen-cli has to match the wasm-bindgen version in Cargo.lock exactly.
RUN set -eux; \
  case "$BUILDARCH" in \
    amd64) bindgen_target=x86_64-unknown-linux-musl; binaryen_target=x86_64-linux ;; \
    arm64) bindgen_target=aarch64-unknown-linux-gnu; binaryen_target=aarch64-linux ;; \
    *) echo "unsupported architecture: $BUILDARCH"; exit 1 ;; \
  esac; \
  bindgen_version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p'); \
  curl -fsSL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$bindgen_version/wasm-bindgen-$bindgen_version-$bindgen_target.tar.gz" \
    | tar -xz --strip-components=1 -C /usr/local/bin "wasm-bindgen-$bindgen_version-$bindgen_target/wasm-bindgen"; \
  curl -fsSL "https://github.com/WebAssembly/binaryen/releases/download/version_$BINARYEN_VERSION/binaryen-version_$BINARYEN_VERSION-$binaryen_target.tar.gz" \
    | tar -xz --strip-components=2 -C /usr/local/bin "binaryen-version_$BINARYEN_VERSION/bin/wasm-opt"; \
  wasm-bindgen --version; wasm-opt --version

COPY src src
COPY assets assets
COPY web/play /out/play

# The loading bar in play/index.html needs the uncompressed size of the wasm,
# since the server sends it gzipped.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
  set -eux; \
  cargo build --locked --profile wasm-release --target wasm32-unknown-unknown; \
  wasm-bindgen --target web --no-typescript --out-dir /out/play --out-name plane_chaos \
    target/wasm32-unknown-unknown/wasm-release/plane_chaos.wasm; \
  wasm-opt -Os --strip-debug \
    --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    -o /out/play/plane_chaos_bg.wasm /out/play/plane_chaos_bg.wasm; \
  cp -r assets /out/play/assets; \
  sed -i "s/__WASM_SIZE__/$(stat -c %s /out/play/plane_chaos_bg.wasm)/" /out/play/index.html; \
  gzip -k -9 /out/play/plane_chaos_bg.wasm /out/play/plane_chaos.js

COPY web/site/manifest.webmanifest /out/site/manifest.webmanifest
COPY web/site/icons /out/site/icons
RUN set -eu; \
  cd /out/play; \
  files=$(find . ../site -type f ! -name '*.gz' ! -name sw.js ! -path ./index.html \
    | sed 's|^\./||; s|^\.\./site/|../|' | sort); \
  build_id=$( (cat index.html; for f in $files; do cat "$(echo "$f" | sed 's|^\.\./|../site/|')"; done) \
    | sha256sum | cut -c1-16); \
  precache=$(printf '"%s",' ./ $files | sed 's/,$//'); \
  sed -i "s|__BUILD_ID__|$build_id|; s|__PRECACHE__|[$precache]|" sw.js; \
  echo "service worker build $build_id: $(echo $files | wc -w) files + page"

# ---- Serve it ----
FROM nginxinc/nginx-unprivileged:1.29-alpine

# Where the landing page's download buttons point to; the native builds are
# attached to GitHub releases by .github/workflows/release.yml.
ENV DOWNLOAD_BASE_URL=https://github.com/derfred/plane_chaos/releases/latest/download

COPY web/nginx.conf.template /etc/nginx/templates/default.conf.template
COPY web/site /usr/share/nginx/html
COPY assets/plane_cabin.png assets/flight_attendant_tray_front.png assets/passenger_*.png assets/icon_*.png \
  assets/coffee_maker_inprogress.png assets/oven_inprogress.png assets/fridge.png assets/taskqueue_panel.png \
  assets/bubble_bg_thought.png \
  /usr/share/nginx/html/img/
COPY --from=game /out/play /usr/share/nginx/html/play

EXPOSE 8080
