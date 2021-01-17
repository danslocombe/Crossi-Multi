rustup run stable-i686-pc-windows-gnu cargo build --release

del "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib\crossy_multi_client.dll"
copy "C:\Users\Dan\crossy_multi\client\target\release\crossy_multi_client.dll" "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib"
del "C:\Users\Dan\crossy_multi\gm\frogger_multi\crossy_multi_client.dll"
copy "C:\Users\Dan\crossy_multi\client\target\release\crossy_multi_client.dll" "C:\Users\Dan\crossy_multi\gm\frogger_multi"