# Dependencies & Kernel Interface Documentation

Dokumen ini memverifikasi pustaka dan antarmuka kernel yang digunakan dalam `memreduct-linux`. Verifikasi dilakukan dengan Context7 dan inspeksi langsung pada sistem Linux CachyOS.

---

## 1. Library & Crates Rust

| Crate | Versi Terverifikasi | Peran / Fungsi | Catatan Arsitektur / API |
|---|---|---|---|
| **`zbus`** | `^5.5` / `^6.1` | D-Bus IPC (Daemon & Client) | Menggunakan macro `#[interface]` untuk server `memreductd` dan `#[proxy]` untuk client `memreduct-gui` & `memreduct-cli`. Berjalan di atas tokio runtime. |
| **`gtk4`** | `^0.11` | GUI Toolkit (Linux Native) | Rust bindings aman untuk GTK4. Mendukung Wayland dan X11 natively. |
| **`libadwaita`** | `^0.9` | Komponen Desain Modern | Menyediakan widget HIG adaptif (`AdwApplicationWindow`, `AdwActionRow`, `AdwPreferencesGroup`, `AdwToastOverlay`). |
| **`ksni`** | `^0.3` | StatusNotifierItem (Tray) | Implementasi D-Bus SNI murni untuk tray di KDE Plasma, GNOME, dan Hyprland/Waybar. |
| **`tokio`** | `^1.43` | Async Runtime | Digunakan oleh daemon untuk event loop D-Bus dan oleh engine auto-reduce timer. |
| **`serde`** & **`toml`** | `^1.0`, `^0.8` | Serialisasi Konfigurasi | Membaca dan menulis konfigurasi TOML `~/.config/memreduct/config.toml`. |
| **`clap`** | `^4.5` | CLI Parser | CLI parser berbasis derive macro untuk `memreduct-cli`. |
| **`tracing`** & **`tracing-subscriber`** | `^0.1`, `^0.3` | Logging & Diagnostik | Output terstruktur yang kompatibel dengan systemd journald. |
| **`thiserror`** | `^2.0` | Error Handling Terstruktur | Definisi tipe error spesifik domain di `memreduct-core` dan daemon. |

---

## 2. Kernel & File System Interfaces

### 2.1 `/proc/meminfo`
- **Tujuan:** Sumber kebenaran metrik memori Linux.
- **Kunci penting:**
  - `MemTotal`: Total RAM fisik (kB).
  - `MemFree`: RAM yang benar-benar belum dialokasikan (kB).
  - `MemAvailable`: Perkiraan RAM yang dapat dialokasikan tanpa memicu swap (kB). Ini adalah metrik terpenting untuk perhitungan utilisasi nyata.
  - `Buffers`: Memori sementara untuk blok disk mentah (kB).
  - `Cached`: Memori page cache untuk pembacaan berkas (kB).
  - `SReclaimable`: Bagian dari kernel Slab cache yang dapat dibebaskan (dentries/inodes) (kB).
  - `SUnreclaim`: Bagian dari kernel Slab cache yang tidak dapat dibebaskan (kB).
  - `SwapTotal` & `SwapFree`: Swap fisik / zram swap space (kB).

### 2.2 `/proc/pressure/memory` (PSI - Pressure Stall Information)
- **Tujuan:** Mengukur penundaan beban kerja akibat tekanan memori nyata (bukan sekadar pembacaan kuantitas memori).
- **Format:**
  ```
  some avg10=0.00 avg60=0.00 avg300=0.00 total=12345
  full avg10=0.00 avg60=0.00 avg300=0.00 total=12345
  ```
- **Kategori:**
  - `some`: Minimal ada satu tugas (task) yang terhenti menunggu ketersediaan memori/IO.
  - `full`: Semua tugas yang dapat berjalan terhenti menunggu memori (kondisi kritis / OOM mendekat).

### 2.3 `/sys/block/zram*/mm_stat`
- **Tujuan:** Statistik internal zram terkompresi di CachyOS.
- **Urutan kolom:**
  1. `orig_data_size` (byte tidak terkompresi)
  2. `compr_data_size` (byte terkompresi di RAM)
  3. `mem_used_total` (total RAM termasuk metadata alokator)
  4. `mem_limit`
  5. `mem_used_max`
  6. `same_pages`
  7. `pages_compacted`
  8. `huge_pages`
  9. `huge_pages_since`
- **Perhitungan rasio kompresi:** `orig_data_size / compr_data_size`.

### 2.4 Control Interfaces (Hak Akses Root / Daemon)
- **`/sys/fs/cgroup/memory.reclaim`** (cgroup v2):
  - Menulis nilai string byte (mis. `500M` atau `1G`) untuk memicu kernel me-reclaim memori secara terkontrol dan aman tanpa membuang cache secara membabi-buta.
- **`/proc/sys/vm/drop_caches`**:
  - `1`: Membebaskan pagecache.
  - `2`: Membebaskan dentries dan inodes (slab cache).
  - `3`: Membebaskan pagecache + dentries + inodes.
  - *Catatan:* Selalu panggil `sync()` sebelum menulis ke berkas ini.
- **`/proc/sys/vm/compact_memory`**:
  - `1`: Meminta defragmentasi dan pemadatan halaman memori kernel.

### 2.5 Polkit D-Bus Authorization
- **Bus:** System Bus (`org.freedesktop.PolicyKit1`)
- **Authority Path:** `/org/freedesktop/PolicyKit1/Authority`
- **Aksi:** `org.memreduct.reduce`
- **Kebijakan Default:** `auth_admin_keep` (meminta otentikasi admin, token disimpan sementara untuk interaksi yang nyaman).
