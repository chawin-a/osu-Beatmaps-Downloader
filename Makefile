.PHONY: release

build:
	cargo build --release
	cp "target\release\downloader.exe" ".\release\osu-Beatmaps-Downloader.exe"
	7z a -tzip osu-Beatmaps-Downloader.zip -w release/. -x!osu-Beatmaps-Downloader.zip && 7z a -tzip osu-Beatmaps-Downloader.zip -r config.example.yaml && mv osu-Beatmaps-Downloader.zip release