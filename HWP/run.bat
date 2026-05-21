@echo off
title HWP Studio Web v2.0
color 0B
echo.
echo  ╔══════════════════════════════════════════╗
echo  ║     HWP Studio Web v2.0                 ║
echo  ║     한국어 문서 편집기  (모든 기기 지원) ║
echo  ╚══════════════════════════════════════════╝
echo.

python --version >nul 2>&1
if errorlevel 1 (
    echo [오류] Python이 설치되어 있지 않습니다.
    echo Python 3.8+ 설치: https://python.org
    pause
    exit /b 1
)

echo [1/2] 패키지 설치 중 (최초 1회)...
pip install fastapi uvicorn python-docx reportlab --quiet --disable-pip-version-check

echo [2/2] HWP Studio 시작 중...
echo.
python server.py

pause
