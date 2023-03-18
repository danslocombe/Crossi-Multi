cd ../web-client
./publish.sh
cd ../web-server
cargo_exe=/home/soweliadmin/.cargo/bin/cargo
key_path=/etc/letsencrypt/live/roadtoads.io/privkey.pem
cert_path=/etc/letsencrypt/live/roadtoads.io/fullchain.pem

~/crossy_multi/web-server/cleanup_logs.sh

sudo $cargo_exe +stable run --release -- ~/crossy_multi/serve $key_path $cert_path
