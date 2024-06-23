# m3u_playlist_browser

Program to easily browse and search very large m3u playlists.

Locally run program to solve the issue of very large m3u playlists not loading well. Implementation of caching mechanisms, lazy loading, pagination, and a search engine to seamlessly browse very large playlists of more than a million entries. For Windows and Linux.

![image](https://github.com/EMRD95/m3u_playlist_browser/assets/114953576/fe4c27db-4c26-48e6-af47-c878874f9195)

## Usage

### On Windows

1. Go to the [release page](https://github.com/EMRD95/m3u_playlist_browser/releases) and download `m3u_browser_Windows.zip`.
2. Extract the folder.
3. Modify `config.txt` to set the path of MPV and/or VLC.
4. Set the path of your m3u playlist or use the one included from [iptv-org/iptv](https://github.com/iptv-org/iptv?tab=readme-ov-file#playlists).
5. Start the program with `run.bat`.
6. Access [http://localhost:8080/](http://localhost:8080/) from your web browser.

### On Linux (GUI needed)

1. Go to the [release page](https://github.com/EMRD95/m3u_playlist_browser/releases) and download `m3u_browser_Linux.zip`.
2. Extract the folder:
   ```sh
   unzip m3u_browser_Linux.zip
   cd m3u_browser_Linux
   ```
3. Install VLC and MPV:
   ```sh
   sudo apt install vlc mpv
   ```
4. Make the binary executable:
   ```sh
   chmod +x m3u_browser
   ```
5. Run the program:
   ```sh
   ./m3u_browser
   ```
6. Access http://localhost:8080/ from your web browser.

## Compilation

### To compile on Windows

Clone the repository and run:
```sh
cargo build --release --target x86_64-pc-windows-msvc
```

### To compile on Linux

```sh
cargo build --release
```
