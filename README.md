# m3u_playlist_browser
Program to easily browse and search very large m3u playlists

Locally run program to solve the issue of very large m3u playlists not loading well. Implementation of caching mechanisms, lazy loading, pagination and a search engine to seamlessly browse very large playlists of more than a million entries. For Windows only right now, easily portable to other OS is requested.



![image](https://github.com/EMRD95/m3u_playlist_browser/assets/114953576/fe4c27db-4c26-48e6-af47-c878874f9195)



To use right now, go to the release page: https://github.com/EMRD95/m3u_playlist_browser/releases

Extract folder.
Modify config.txt to set the path of your MPV and/or VLC path.
Set the path of your m3u playlist or use the one included from https://github.com/iptv-org/iptv?tab=readme-ov-file#playlists
Start the program with run.bat
Access http://localhost:8080/ from your web browser.

To compile, clone the repository

cargo build --release --target x86_64-pc-windows-msvc
