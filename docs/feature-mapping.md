# Feature Mapping: Mem Reduct (Windows) → Linux (`memreduct-linux`)

Analisis komparatif reverse-engineering terhadap repositori `henrypp/memreduct` (C, Win32/NT Native API) dan perancangan padanannya pada platform Linux modern (khususnya CachyOS / Arch-based).

---

## 1. Arsitektur Inti & Hak Akses

| Komponen | Mem Reduct (Windows) | `memreduct-linux` (Linux) | Catatan / Risiko Linux |
|---|---|---|---|
| **Model Hak Akses** | Single process; meminta UAC elevation (`runas`) saat startup jika ingin fitur reduce aktif | **Privilege Separation**: GUI/CLI berjalan sebagai user biasa; daemon `memreductd` berjalan sebagai root via systemd & D-Bus system bus | GUI tidak pernah memegang akses root. Mengurangi *attack surface* secara drastis. |
| **Otorisasi Tindakan** | Token hak istimewa Windows (`SeDebugPrivilege`, `SeIncreaseQuotaPrivilege`, dll) | **Polkit** (`org.memreduct.reduce`) dengan dialog otorisasi pengguna interaktif | Pengguna dikonfirmasi via dialog agen Polkit desktop (KDE/GNOME). |
| **IPC** | Single binary (Win32 message loop & shared memory) | **D-Bus System Bus** (`org.memreduct.Daemon`) via library `zbus` | Protokol standar Linux, aman, mendukung aktivasi bus dan audit polkit. |

---

## 2. Pemantauan Memori (Memory Monitoring)

| Metrik Windows (`memreduct`) | API Windows Asli | Padanan Linux | Sumber Data Linux | Catatan Semantik |
|---|---|---|---|---|
| **Physical Memory (RAM)** | `GlobalMemoryStatusEx` & `NtQuerySystemInformation` | RAM Total, Terpakai, Bebas, Tersedia | `/proc/meminfo` (`MemTotal`, `MemFree`, `MemAvailable`) | Di Linux, gunakan **`MemAvailable`** sebagai acuan utama, bukan `MemFree`, karena kernel memakai RAM menganggur untuk cache. |
| **Pagefile / Swap** | `MEMORYSTATUSEX.ullTotalPageFile` | Swap / zram | `/proc/swaps`, `/proc/meminfo` (`SwapTotal`, `SwapFree`) | CachyOS menggunakan zram secara default. |
| **zram Compression** | *Tidak ada di Windows* | Ukuran terkompresi, rasio kompresi, total memori | `/sys/block/zram*/mm_stat` | Menampilkan metrik unik CachyOS: `orig_data_size`, `compr_data_size`, kompresi rasio. |
| **System Cache / Buffer** | `SystemFileCacheInformation` | Buffers, Page Cache, Slab | `/proc/meminfo` (`Buffers`, `Cached`, `SReclaimable`, `SUnreclaim`) | Cache di Linux bersifat dinamis dan sehat; ditampilkan terpisah dari anon memory. |
| **Memory Pressure (PSI)** | *Tidak ada padanan langsung* | Pressure Stall Information | `/proc/pressure/memory` (`some`, `full` avg10/avg60/avg300) | Indikator objektif apakah CPU/IO terhambat oleh kekurangan memori. |
| **Top Processes** | Windows Process Snapshot API | Top proses berdasarkan PSS/RSS | `/proc/[pid]/statm`, `/proc/[pid]/status` | Opsional, dibaca langsung oleh GUI tanpa perlu hak akses root. |

---

## 3. Wilayah & Operasi Pembersihan (Memory Reduction)

Di Mem Reduct asli (`main.h`, `main.c`), pembersihan dikendalikan oleh bitmask (`REDUCT_MASK_*`):

| Mask Windows | API Windows Asli | Padanan Linux | Mekanisme Eksekusi di Daemon | Dampak & Risiko di Linux |
|---|---|---|---|---|
| `REDUCT_WORKINGSET` (0x01) | `NtSetSystemInformation(MemoryEmptyWorkingSets)` | **cgroup v2 `memory.reclaim`** | Tulis byte target (mis. `500M`) ke `/sys/fs/cgroup/memory.reclaim` | **Aman & Modern.** Kernel Linux mereclaim halaman dingin tanpa mematikan proses atau mengosongkan filesystem cache. |
| `REDUCT_SYSTEMFILECACHE` (0x02) | `NtSetSystemInformation(SystemFileCacheInformationEx)` | **Drop Pagecache** | `sync(); echo 1 > /proc/sys/vm/drop_caches` | Membersihkan pagecache yang sudah di-flush ke disk. **Efek samping:** I/O disk berikutnya akan sedikit lebih lambat saat membaca kembali file. |
| `REDUCT_MODIFIEDFILECACHE` (0x80) | `_app_flushvolumecache()` (`_r_fs_flushfile`) | **Sync Disk Buffers** | `sync()` syscall | Sangat aman. Menulis dirty pages dari RAM ke disk/storage. |
| `REDUCT_MODIFIEDLIST` (0x10) | `NtSetSystemInformation(MemoryFlushModifiedList)` | `sync()` | Syscall `libc::sync()` | Menjamin dirty pages tersinkronisasi. |
| `REDUCT_STANDBYLIST` (0x08) & `PRIORITY0` (0x04) | `NtSetSystemInformation(MemoryPurgeStandbyList)` | **Drop Caches (Pagecache + Dentries/Inodes)** | `sync(); echo 3 > /proc/sys/vm/drop_caches` | Membersihkan semua cache kernel. Hanya disarankan bila memori fisik benar-benar kritis. |
| `REDUCT_REGISTRYCACHE` (0x40) | `NtSetSystemInformation(SystemRegistryReconciliationInformation)` | **Drop Dentries & Inodes (Slab)** | `sync(); echo 2 > /proc/sys/vm/drop_caches` | Membersihkan dentry dan inode cache di kernel slab (`SReclaimable`). |
| `REDUCT_COMBINEMEMORYLISTS` (0x20) | `NtSetSystemInformation(SystemCombinePhysicalMemoryInformation)` | **Compact Memory** | `echo 1 > /proc/sys/vm/compact_memory` | Mendefragmentasi halaman memori fisik kernel (mengurangi fragmentasi memori). |
| *Swapoff/Swapon* | *Tidak ada* | **Tidak didukung otomatis** | Tampilkan tombol manual dengan konfirmasi ketat | Risiko OOM kill bila RAM fisik tersisa < swap yang terpakai. |

---

## 4. Fitur UI, Tray, & Otomatisasi

| Fitur Windows (`memreduct`) | Implementasi Windows | Implementasi Linux (`memreduct-linux`) |
|---|---|---|
| **System Tray Icon** | Win32 `Shell_NotifyIcon` + dynamic GDI bitmap (angka persentase digambar ke HBITMAP) | **StatusNotifierItem (SNI / D-Bus)** via crate `ksni` (didukung native oleh KDE Plasma & GNOME via AppIndicator) |
| **Tray Tooltip** | `_r_tray_setinfoformat` (RAM, Pagefile, Cache) | SNI Tooltip dengan format data RAM, Swap/zram, dan PSI status |
| **Dynamic Tray Color** | Normal (Hijau), Warning (Oranye), Danger (Merah) | Ikon SVG / pixmap dinamis sesuai ambang batas `TrayLevelWarning` dan `TrayLevelDanger` |
| **Auto-reduct by Threshold** | Timer polling 1s: jika RAM% >= ambang batas (default 90%) dan cooldown >= 30 detik | Polling timer tokio (1s): periksa `MemAvailable%` < ambang atau PSI avg10 > ambang, dengan cooldown minimum 5 menit (default mati) |
| **Auto-reduct by Interval** | Timer: bersihkan setiap N menit | Interval timer konfigurabel (default mati) |
| **Global Hotkey** | `RegisterHotKey` (default Ctrl+F1) | Shortcut desktop standar via D-Bus method atau `.desktop` action yang dapat di-bind di KDE Shortcuts / GNOME Keyboard Shortcuts |
| **Hasil Pembersihan** | Balon notifikasi / MessageBox: selisih bytes sebelum dan sesudah | Desktop notification (Freedesktop Notifications via D-Bus / `notify-rust` / `libadwaita` in-app toast) |
| **Konfigurasi** | Registry / `memreduct.ini` | File TOML di `~/.config/memreduct/config.toml` |
