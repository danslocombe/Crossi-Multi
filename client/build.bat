REM rustup run stable-i686-pc-windows-gnu cargo build --release

REM del "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib\crossy_multi_client.dll"
REM copy "C:\Users\Dan\crossy_multi\client\target\release\crossy_multi_client.dll" "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib"
REM del "C:\Users\Dan\crossy_multi\gm\frogger_multi\crossy_multi_client.dll"
REM copy "C:\Users\Dan\crossy_multi\client\target\release\crossy_multi_client.dll" "C:\Users\Dan\crossy_multi\gm\frogger_multi"

rustup run stable-i686-pc-windows-gnu cargo build

del "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib\crossy_multi_client.dll"
del "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib\crossy_multi_client.pdb"
copy "C:\Users\Dan\crossy_multi\client\target\debug\crossy_multi_client.dll" "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib"
copy "C:\Users\Dan\crossy_multi\client\target\debug\crossy_multi_client.pdb" "C:\Users\Dan\Documents\GameMaker\Projects\frogger_multi.gmx\extensions\frog_mult_lib"
del "C:\Users\Dan\crossy_multi\gm\frogger_multi\crossy_multi_client.dll"
del "C:\Users\Dan\crossy_multi\gm\frogger_multi\crossy_multi_client.pdb"
copy "C:\Users\Dan\crossy_multi\client\target\debug\crossy_multi_client.dll" "C:\Users\Dan\crossy_multi\gm\frogger_multi"
copy "C:\Users\Dan\crossy_multi\client\target\debug\crossy_multi_client.pdb" "C:\Users\Dan\crossy_multi\gm\frogger_multi"