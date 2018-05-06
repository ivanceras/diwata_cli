cd ../elm-webclient/

./release_compile.sh

cd ../

rsync -vahP --delete public/ ./cli/public/

cd cli/

rsync -vahP ./static/ ./public/static/

inline-assets ./public/static/index-cli.html ./public/static/inline-cli.html
