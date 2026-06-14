@echo off
chcp 65001 > nul
setlocal
set RCEDIT="C:\Users\Administrator\AppData\Roaming\QClaw\npm-global\node_modules\rcedit\bin\rcedit-x64.exe"
set EXE=D:\WorkSpace\DesktopOrganizer\target\release\desktop-organizer.exe
set ICO=D:\WorkSpace\DesktopOrganizer\assets\app_icon.ico
set OUT=D:\WorkSpace\DesktopOrganizer\release\DesktopOrganizer.exe
set VER=D:\WorkSpace\DesktopOrganizer\release\version.txt

if not exist "%RCEDIT%" (
  echo [ERROR] rcedit not found at %RCEDIT%
  pause
  exit /b 1
)
if not exist "%EXE%" (
  echo [ERROR] exe not found at %EXE%
  echo         Did you run 'cargo build --release' first?
  pause
  exit /b 1
)
if not exist "%ICO%" (
  echo [ERROR] ico not found at %ICO%
  pause
  exit /b 1
)

echo [1/2] Copying %EXE% ^-^> %OUT%
copy /Y "%EXE%" "%OUT%" > nul
if errorlevel 1 (
  echo [ERROR] copy failed
  pause
  exit /b 1
)

echo [2/2] Injecting icon...
%RCEDIT% "%OUT%" --set-icon "%ICO%"
set RC=%ERRORLEVEL%
echo rcedit exit code: %RC%
if not "%RC%"=="0" (
  echo [ERROR] rcedit failed
  pause
  exit /b 1
)

rem Write version stamp for verification
for /f "tokens=*" %%v in ('powershell -NoProfile -Command "(Get-Item '%OUT%').LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss') + ' size=' + (Get-Item '%OUT%').Length"') do echo %%v> "%VER%"
type "%VER%"

echo.
echo Done. Final exe: %OUT%
echo.
pause
