build:
	cargo build --release
	cp "target\release\downloader.exe" ".\release\osu!-Beatmaps-Downloader.exe"