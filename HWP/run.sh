#!/bin/bash
echo "HWP Studio Web v2.0 – 한국어 문서 편집기"
echo "패키지 설치 중..."
pip install fastapi uvicorn python-docx reportlab --quiet
echo "HWP Studio 시작 중..."
python3 server.py
