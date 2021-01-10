rustup run stable-i686-pc-windows-gnu cargo build --release

del "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib\crossy_multi.dll"
copy "C:\Users\Dan\crossy_multi\target\release\crossy_multi.dll" "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib"