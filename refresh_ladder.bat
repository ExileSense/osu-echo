@echo off
if not exist ".data\ladder" mkdir ".data\ladder"
curl.exe -fL -A "osu-echo-local (personal use; at most one request/day)" -o ".data\ladder\osu.dataset.tmp" "https://osu-api-bridge.ameo.dev/analysis/dataset?mode=0"
if errorlevel 1 (echo Download failed, keeping the current dataset. & exit /b 1)
if exist ".data\ladder\osu.dataset" copy /Y ".data\ladder\osu.dataset" ".data\ladder\osu.dataset.prev" >nul
move /Y ".data\ladder\osu.dataset.tmp" ".data\ladder\osu.dataset" >nul
echo Done. The server picks up the new file automatically.
