# mov2mp4

아이폰 등의 MOV 파일을 MP4로, HEIC/HEIF 사진을 JPG로 변환하는 Windows 명령줄 프로그램입니다. 출력 파일은 원본과 같은 폴더에 만듭니다.

## 빌드 준비

- Rust 도구 체인 설치: <https://rustup.rs/>
- Windows SDK의 리소스 컴파일러(`rc.exe`)

FFmpeg과 HEIF 변환 도구는 실행 파일에 포함되어 있습니다. 사용하는 PC에는 Rust나 변환 도구를 따로 설치할 필요가 없습니다.

실행 파일 아이콘은 검은 배경에 회색 `MOV`와 흰색 `MP4`를 두 줄로 배치한 [`assets/mov2mp4.ico`](assets/mov2mp4.ico)입니다. 16~256px 크기를 각각 그려 작은 아이콘의 글자 형태를 보존합니다. 큰 미리보기는 [`assets/mov2mp4.png`](assets/mov2mp4.png)입니다. 아이콘을 다시 만들려면 Python과 Pillow로 `python tools/generate_icon.py`를 실행하세요.

## 빌드

```powershell
cargo build --release
```

실행 파일은 `target\release\mov2mp4.exe`에 생성됩니다. 작업 폴더의 `mov2mp4.exe`는 배포용 복사본입니다. 다른 Windows PC에는 이 파일 하나만 복사하면 됩니다. 실행 파일 크기는 약 81MiB입니다. 필요한 변환 도구는 실행 시 임시 폴더에 풀어 사용한 뒤 삭제합니다.

## 사용

```powershell
mov2mp4.exe a.mov
mov2mp4.exe *.mov
mov2mp4.exe "C:\Videos\*.mov"
mov2mp4.exe photo.heic
mov2mp4.exe photo.heif
mov2mp4.exe *.heic
mov2mp4.exe *.heif
mov2mp4.exe "C:\Photos\*.heic"
```

파일 경로 하나는 해당 파일을 처리합니다. `*.mov`, `*.heic`, `*.heif`는 현재 폴더에서 해당 확장자의 파일을 모두 처리합니다. `C:\Photos\*.heic`처럼 폴더도 지정할 수 있습니다. 하위 폴더는 검색하지 않으며 `.MOV`, `.HEIC`, `.HEIF`처럼 대문자 확장자도 포함합니다. 원본은 유지하고 MOV는 MP4, HEIC/HEIF는 JPG를 만듭니다. 기존 출력 파일은 덮어쓰지 않습니다. 한 파일이 실패해도 나머지 파일은 계속 처리하며, 변환이 실패하면 해당 임시 파일을 삭제합니다.

성공하면 `변환 : a.mov → a.mp4 [3.55 초]` 형식으로 한 줄만 출력합니다.

영상과 음성을 다시 인코딩하지 않고 MP4 컨테이너로 옮기므로 빠르고 화질 손실이 없습니다. 다만 입력 영상/음성 코덱이 MP4와 호환되지 않으면 변환이 실패할 수 있습니다. 이 프로그램은 첫 번째 영상 스트림과 모든 음성 스트림을 사용합니다.

HEIC/HEIF는 JPG 품질 92로 변환합니다. JPEG로 변환할 때는 압축에 따른 화질 손실이 있으며, 투명도와 일부 HEIF 고유 기능은 JPG에 담을 수 없습니다. 사진에 EXIF 정보가 있으면 변환 도구가 이를 JPG로 옮깁니다.

## 포함된 도구와 라이선스

내장 FFmpeg은 GPLv3 빌드입니다. 라이선스 전문은 `mov2mp4.exe --license` 명령이나 [`vendor/FFMPEG_LICENSE.txt`](vendor/FFMPEG_LICENSE.txt)에서 확인할 수 있습니다. 이 실행 파일을 다른 사람에게 배포할 경우 FFmpeg의 라이선스와 해당 빌드의 소스 코드 제공 의무를 확인해야 합니다. FFmpeg의 [라이선스 안내](https://ffmpeg.org/legal.html)를 참고하세요.

HEIC/HEIF 변환에는 [libheif](https://github.com/strukturag/libheif)의 Windows 빌드와 `heif-dec`를 포함합니다. libheif 라이브러리는 LGPL, 예제 프로그램은 MIT 라이선스입니다. 라이선스 전문은 [`vendor/LIBHEIF_LICENSE.txt`](vendor/LIBHEIF_LICENSE.txt)와 `mov2mp4.exe --license`에서 확인할 수 있습니다. 압축 해제에는 [7-Zip](https://www.7-zip.org)의 `7zr.exe`를 포함합니다. 재배포할 때는 함께 포함된 코덱 라이브러리의 라이선스도 확인하세요.
