cargo_exe=/home/soweliadmin/.cargo/bin/cargo
key_path=/etc/letsencrypt/live/roadtoads.io/privkey.pem
cert_path=/etc/letsencrypt/live/roadtoads.io/fullchain.pem
RUSTFLAGS="--cfg tokio_unstable"
echo $RUSTFLAGS
sudo $cargo_exe +stable run --release -- ~/crossy_multi/serve $key_path $cert_path
#sudo $cargo_exe run --release -- ~/crossy_multi/serve $key_path $cert_path
