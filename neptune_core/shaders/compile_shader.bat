for /r %%i in (*.vert *frag) do glslc.exe %%~nxi -o %%~nxi.spv

pause