cd ../elm-webclient/

./release_compile.sh

cd ../

mkdir -p cli/public

rsync -vahP --delete public/ ./cli/public/

