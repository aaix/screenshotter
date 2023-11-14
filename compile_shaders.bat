@echo off
IF NOT exist "compiled_shaders/" mkdir compiled_shaders
fxc /T vs_5_0 /Fo ./compiled_shaders/VertexShader.cso /E VS_main Shaders.hlsl /nologo /Zi
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T ps_5_0 /Fo ./compiled_shaders/PixelShader.cso /E PS_main Shaders.hlsl /nologo /Zi
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T cs_5_0 /Fo ./compiled_shaders/ConvertShader.cso /E CS_convert_main Shaders.hlsl /nologo /Zi
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
fxc /T cs_5_0 /Fo ./compiled_shaders/PreprocessShader.cso /E CS_preprocess_main Shaders.hlsl /nologo /Zi
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
EXIT /B 0