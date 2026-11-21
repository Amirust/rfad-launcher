@echo off
rem Builds the launcher, signs it and prepares everything for a release in .\deploy
rem
rem   deploy.bat
rem
rem Key path:      %SIGN_KEY% (default: private.key)
rem Key password:  %SIGN_KEY_PASSWORD%, otherwise asked interactively (must not be empty)
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"

set "REPO=Amirust/rfad-launcher"
set "EXE_NAME=rfad-launcher.exe"
set "OUT=deploy"
if not defined SIGN_KEY set "SIGN_KEY=private.key"

if not exist "%SIGN_KEY%" (
  echo ERROR: private key not found: %SIGN_KEY%
  exit /b 1
)

rem --- versions ---------------------------------------------------------------
set "VERSION="
set "CARGO_VERSION="
for /f "usebackq delims=" %%v in (`powershell -NoProfile -Command "(Get-Content -Raw 'src-tauri\tauri.conf.json' | ConvertFrom-Json).version"`) do set "VERSION=%%v"
for /f "usebackq delims=" %%v in (`powershell -NoProfile -Command "(Select-String -Path 'src-tauri\Cargo.toml' -Pattern '^version = .(.+).$' | Select-Object -First 1).Matches[0].Groups[1].Value"`) do set "CARGO_VERSION=%%v"

if not defined VERSION (
  echo ERROR: cannot read version from tauri.conf.json
  exit /b 1
)
if not "%VERSION%"=="%CARGO_VERSION%" (
  echo ERROR: version mismatch: tauri.conf.json=%VERSION%, Cargo.toml=%CARGO_VERSION%
  exit /b 1
)

rem --- password ---------------------------------------------------------------
rem tauri signer waits for keyboard input when no password is given, so it is
rem always passed through TAURI_PRIVATE_KEY_PASSWORD.
if defined SIGN_KEY_PASSWORD (
  set "TAURI_PRIVATE_KEY_PASSWORD=%SIGN_KEY_PASSWORD%"
  goto :have_password
)
set "TAURI_PRIVATE_KEY_PASSWORD="
for /f "usebackq delims=" %%p in (`powershell -NoProfile -Command "$s = Read-Host 'Signing key password' -AsSecureString; [Runtime.InteropServices.Marshal]::PtrToStringBSTR([Runtime.InteropServices.Marshal]::SecureStringToBSTR($s))"`) do set "TAURI_PRIVATE_KEY_PASSWORD=%%p"
:have_password
if not defined TAURI_PRIVATE_KEY_PASSWORD (
  echo ERROR: empty password. Keys without a password are not supported by deploy.bat, use deploy.sh
  exit /b 1
)

echo ==^> Checking key password
set "TEST_FILE=%TEMP%\rfad-sign-test-%RANDOM%.txt"
echo test> "%TEST_FILE%"
call :sign "%TEST_FILE%" || goto :fail
del /q "%TEST_FILE%" "%TEST_FILE%.sig" >nul 2>&1

rem --- tests and build --------------------------------------------------------
echo ==^> Running tests
pushd src-tauri
cargo test --lib --quiet
if errorlevel 1 (
  popd
  echo ERROR: tests failed
  goto :fail
)
popd

echo ==^> Building v%VERSION%
call bunx tauri build --no-bundle
if errorlevel 1 (
  echo ERROR: build failed
  goto :fail
)

rem --- deploy folder ----------------------------------------------------------
echo ==^> Preparing %OUT%\
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"
copy /y "src-tauri\target\release\%EXE_NAME%" "%OUT%\%EXE_NAME%" >nul || goto :fail

echo ==^> Signing %EXE_NAME%
call :sign "%OUT%\%EXE_NAME%" || goto :fail

set "SHA256="
for /f "usebackq delims=" %%h in (`powershell -NoProfile -Command "(Get-FileHash -Algorithm SHA256 '%OUT%\%EXE_NAME%').Hash.ToLower()"`) do set "SHA256=%%h"
if not defined SHA256 (
  echo ERROR: cannot compute sha256
  goto :fail
)

set "URL=https://github.com/%REPO%/releases/download/v%VERSION%/%EXE_NAME%"
set "MANIFEST=%CD%\%OUT%\lversions.json"

echo ==^> Writing and signing lversions.json
rem LF line endings, UTF-8 without BOM: the signature covers these exact bytes
powershell -NoProfile -Command "$q=[char]34; $n=[char]10; $s='{0}{1}  {2}version{2}: {2}{3}{2},{1}  {2}downloadUrl{2}: {2}{4}{2},{1}  {2}sha256{2}: {2}{5}{2}{1}{6}{1}' -f '{',$n,$q,$env:VERSION,$env:URL,$env:SHA256,'}'; [IO.File]::WriteAllText($env:MANIFEST, $s)" || goto :fail
call :sign "%OUT%\lversions.json" || goto :fail

rem The manifest the launchers read lives in the repo root
copy /y "%OUT%\lversions.json" "lversions.json" >nul || goto :fail
copy /y "%OUT%\lversions.json.sig" "lversions.json.sig" >nul || goto :fail

echo.
echo Done: v%VERSION%
echo   %OUT%\%EXE_NAME%
echo   %OUT%\%EXE_NAME%.sig
echo   %OUT%\lversions.json        (also copied to repo root)
echo   %OUT%\lversions.json.sig    (also copied to repo root)
echo   sha256: %SHA256%
echo.
echo Release order:
echo   1. GitHub release with tag v%VERSION% -^> upload %EXE_NAME% and %EXE_NAME%.sig
echo   2. Only after that commit and push lversions.json + lversions.json.sig to main
exit /b 0

rem --- helpers ----------------------------------------------------------------
:sign
call bunx tauri signer sign -f "%SIGN_KEY%" "%~1" >nul
if errorlevel 1 (
  echo ERROR: signing %~1 failed ^(wrong password?^)
  exit /b 1
)
if not exist "%~1.sig" (
  echo ERROR: signature %~1.sig was not created
  exit /b 1
)
exit /b 0

:fail
if defined TEST_FILE del /q "%TEST_FILE%" "%TEST_FILE%.sig" >nul 2>&1
echo.
echo DEPLOY FAILED
exit /b 1
