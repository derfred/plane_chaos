#!/bin/sh
# Renders the web app icons (home screen, browser tab) from the game's sprites
# into web/site/icons/. Needs ImageMagick 7. Run from the repository root:
#
#   sh web/icons/make-icons.sh
set -eu

out=web/site/icons
mkdir -p "$out"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

sky="#87bfeb"
aisle="#c44848"
stripe="#f2b84b"

# 1024px master: sky with a cloud, the red aisle running off the corner, the
# attendant with her tray and a thought bubble asking for coffee.
magick -size 1024x1024 "xc:$sky" \
  -fill "#ffffffaa" -draw "ellipse 250 250 200 70 0 360" -draw "ellipse 360 210 120 60 0 360" \
  -fill "$aisle" -draw "polygon 380,1024 1024,560 1024,760 660,1024" \
  -stroke "$stripe" -strokewidth 14 -fill none \
  -draw "line 590,960 640,924" -draw "line 730,860 780,824" -draw "line 870,760 920,724" \
  "$tmp/background.png"

magick assets/flight_attendant_tray_front.png -filter Lanczos -resize x780 "$tmp/attendant.png"
magick assets/bubble_bg_thought.png -filter Lanczos -resize x300 "$tmp/bubble.png"
magick assets/icon_desire_coffee.png -filter Lanczos -resize 150x150 "$tmp/coffee.png"

magick "$tmp/background.png" \
  "$tmp/attendant.png" -gravity south -geometry -60+40 -composite \
  "$tmp/bubble.png" -gravity northeast -geometry +120+90 -composite \
  "$tmp/coffee.png" -gravity northeast -geometry +162+129 -composite \
  -alpha off "$tmp/master.png"

# iOS rounds the corners itself and turns transparency black, so the icons
# are opaque squares. The maskable icon (Android) keeps the art inside the
# central 80% safe zone.
magick "$tmp/master.png" -filter Lanczos -resize 180x180 "$out/apple-touch-icon.png"
magick "$tmp/master.png" -filter Lanczos -resize 192x192 "$out/icon-192.png"
magick "$tmp/master.png" -filter Lanczos -resize 512x512 "$out/icon-512.png"
magick "$tmp/master.png" -filter Lanczos -resize 410x410 -background "$sky" -gravity center \
  -extent 512x512 "$out/icon-maskable-512.png"
magick "$tmp/master.png" -filter Lanczos -resize 32x32 "$out/favicon-32.png"
