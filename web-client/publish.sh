set -e
npm run build
echo Build finished, deploying
rm -rf ../serve/*
cp -r dist/* ../serve
cp -r static/* ../serve
echo Done
