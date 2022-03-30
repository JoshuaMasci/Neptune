for /r %%i in (*.vert *frag) do C:/VulkanSDK/1.3.204.0/Bin/glslc.exe %%~nxi -o %%~nxi.spv

pause