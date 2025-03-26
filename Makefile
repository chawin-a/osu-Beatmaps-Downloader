.PHONY: release

run:
	RUST_LOG=info cargo run

build:
	cargo build --release

release:
	cargo build --release
	cp "target\release\downloader.exe" ".\release\osu-Beatmaps-Downloader.exe"
	7z a -tzip osu-Beatmaps-Downloader.zip -w release/. -x!osu-Beatmaps-Downloader.zip -x!config.yaml && 7z a -tzip osu-Beatmaps-Downloader.zip -r config.example.yaml && mv osu-Beatmaps-Downloader.zip release