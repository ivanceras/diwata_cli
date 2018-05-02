cd ../elm-webclient/

./release_compile.sh

cd ../

rsync -vahP --delete public/ ./diwata_cli/public/

cd diwata_cli/

rsync -vahP ./static/ ./public/static/

inline-assets ./public/static/index-cli.html ./public/static/inline-cli.html
