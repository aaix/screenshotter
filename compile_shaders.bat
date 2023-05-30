@echo off
IF NOT exist "compiled_shaders/" mkdir compiled_shaders
fxc /T vs_5_0 /Fo ./compiled_shaders/VertexShader.cso /E VS_main Shaders.hlsl
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T ps_5_0 /Fo ./compiled_shaders/PixelShader.cso /E PS_main Shaders.hlsl
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T cs_5_0 /Fo ./compiled_shaders/ConvertShader.cso /E CS_convert_main Shaders.hlsl
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T cs_5_0 /Fo ./compiled_shaders/MinMaxShader.cso /E CS_minmax_main Shaders.hlsl
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
EXIT /B 0