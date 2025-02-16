@echo off

REM Make sure the git submodules are up to date
git submodule update --init --recursive     
git submodule update --remote

REM Build llama
cd llama
mkdir build
cd build
cmake ..
cmake --build . --config Release
cd ..
mkdir ..\src-tauri\lib
mkdir ..\src-tauri\bin
copy .\build\bin\Release\server.exe ..\src-tauri\bin\llm-server-x86_64-pc-windows-msvc.exe