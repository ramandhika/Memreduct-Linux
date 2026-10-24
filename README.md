# memreduct-linux

> **Real-time Memory Monitor & Optimizer untuk CachyOS / Linux**
> Terinspirasi dari konsep [henrypp/memreduct](https://github.com/henrypp/memreduct) di Windows, didesain ulang dari nol untuk arsitektur kernel Linux modern.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](Cargo.toml)
[![Target: CachyOS / Arch Linux](https://img.shields.io/badge/Platform-CachyOS%20%7C%20Arch%20Linux-brightgreen.svg)](https://cachyos.org)

---

## 🇮🇩 Bahasa Indonesia

### 1. Filosofi & Realitas Teknis (PENTING)

Linux **bukan** Windows. Manajemen memori kernel Linux memperlakukan "RAM yang tidak terpakai adalah RAM yang terbuang" (*Unused RAM is wasted RAM*). Kernel Linux secara agresif memanfaatkan sisa RAM sebagai **Page Cache** dan **Buffers** untuk mempercepat I/O disk, dan akan melepaskannya secara otomatis dan instan saat ada aplikasi yang membutuhkan RAM.

#### Kapan Memory Reduction Berguna?
- Saat hendak meluncurkan game berat atau VM/emulatore dan ingin memaksimalkan contiguous memory / headroom awal.
- Saat aplikasi berat (IDE, browser tab ratusan, compiler) baru saja ditutup dan meninggalkan dirty pages atau cached slabs dalam jumlah masif.
- Saat ingin mendefrag memory pages (`compact_memory`) untuk mengurangi fragmentasi memori sebelum alokasi HugePages.
- Melalui cgroup v2 `memory.reclaim` (mode default): memaksa sistem memangkas alokasi non-kritis secara halus tanpa membabat seluruh cache disk sistem.

#### Kapan Memory Reduction TIDAK Berguna (atau Merugikan)?
- **Membersihkan cache secara terus-menerus (`drop_caches` loop):** justru memperlambat sistem dan meningkatkan disk read IOPS karena kernel harus membaca ulang file binary dan library dari SSD/HDD.
- **Mengejar "RAM Usage 10%":** metrik kesehatan Linux yang benar adalah **`MemAvailable`** dan **PSI (Pressure Stall Information)**, bukan `MemFree`.

`memreduct-linux` memegang prinsip **Monitor-First**: menyajikan data jujur (`MemAvailable`, zram compression ratio, PSI pressure), menyediakan opsi pembersihan yang aman secara bertingkat, dan menonaktifkan auto-reduce secara default dengan proteksi cooldown.

---

### 2. Fitur Utama

- 📊 **Real-time Memory Monitoring:**
  - RAM Total, Terpakai, `MemAvailable`, Cache & Buffer (akurasi 100% dari `/proc/meminfo`).
  - Swap & **zram CachyOS** terpisah: ukuran asli vs terkompresi, rasio kompresi (mis. 2.8x), dan bytes yang berhasil dihemat.
  - **PSI (Pressure Stall Information):** Memantau indikator tekanan memori kernel riil (`some` dan `full` stall).
  - Grafik riwayat dinamis 60 detik berbasis Cairo drawing tanpa dependensi library eksternal yang berat.
- 🧹 **Tingkatan Reduksi (Privilege Separated):**
  1. `reclaim` (Default & Paling Aman): Menggunakan cgroup v2 memory reclaim + `sync`.
  2. `pagecache`: `sync` diikuti pembersihan page cache (`echo 1 > drop_caches`).
  3. `slab`: Membersihkan dentries dan inodes (`echo 2 > drop_caches`).
  4. `all`: Membersihkan page cache, dentries, dan inodes (`echo 3 > drop_caches`). Memerlukan konfirmasi pengguna.
  5. `compact`: Defragmentasi memori kernel via `compact_memory`.
- 🛡️ **Arsitektur Aman (Privilege Separation):**
  - GUI dan CLI berjalan sebagai user biasa tanpa hak akses root.
  - Daemon (`memreductd`) berjalan sebagai systemd service berhak akses root, berkomunikasi via D-Bus (`org.memreduct.Daemon`), dan diawasi oleh kebijakan otorisasi **Polkit** (`org.memreduct.reduce`).
  - GUI tetap berfungsi sebagai monitor saat daemon tidak aktif (*Graceful Degradation*).
- 📌 **System Tray Universal (KDE Plasma, GNOME, Hyprland, Sway):**
  - Menggunakan StatusNotifierItem (`ksni`) native D-Bus dengan indikator persentase real-time dan quick menu.
- ⚙️ **Auto-Reduce & Cooldown Proteksi:**
  - Pemicu otomatis berbasis ambang batas RAM %, PSI memory stall, atau interval waktu.
  - Cooldown bawaan (default 5 menit) untuk mencegah *thrashing*.

---

### 3. Instalasi di CachyOS / Arch Linux

#### Menggunakan PKGBUILD:
```bash
git clone https://github.com/memreduct-linux/memreduct-linux.git
cd memreduct-linux/packaging/arch
makepkg -si
```

#### Kompilasi Manual dari Source:
```bash
# Pastikan dependensi terpasang
sudo pacman -S --needed base-devel rust cargo gtk4 libadwaita polkit dbus

# Build workspace
cargo build --release --locked --workspace

# Jalankan CLI
./target/release/memreduct-cli status

# Jalankan GUI
./target/release/memreduct-gui
```

#### Menjalankan & Mengaktifkan Daemon:
```bash
sudo systemctl enable --now memreductd.service
```

---

### 4. Penggunaan CLI (`memreduct-cli`)

CLI dapat digunakan untuk inspeksi cepat atau otomasi script:

```bash
# Menampilkan status memori ringkas
memreduct-cli status

# Menampilkan output format JSON
memreduct-cli status --json

# Menjalankan pembersihan memori (meminta otorisasi Polkit jika lewat daemon)
memreduct-cli reduce --mode reclaim

# Mode agresif
memreduct-cli reduce --mode pagecache
```

---

## 🇬🇧 English

### 1. Philosophy & Linux Memory Reality

Linux memory management follows the principle: **"Unused RAM is wasted RAM"**. The Linux kernel aggressively uses spare memory as **Page Cache** and **Buffers** to accelerate disk operations. The kernel frees this cache automatically whenever an application requires memory.

#### When is Memory Reduction Useful?
- Prior to launching large memory-hungry workloads (games, VMs, local LLMs).
- Right after closing large applications that filled cache with dirty/unneeded pages.
- Memory compaction (`compact_memory`) to reduce page fragmentation before allocating HugePages.
- Gentle process trimming via cgroup v2 `memory.reclaim`.

#### When is it NOT Useful?
- Aggressive repetitive dropping of caches (`drop_caches` loops), which degrades performance by forcing disk read I/O.
- True health metrics are **`MemAvailable`** and **PSI (Pressure Stall Information)**, not `MemFree`.

`memreduct-linux` is **Monitor-First**: provides transparent metrics, safe reduction modes, and disables auto-reduction by default.

---

### 2. Architecture

```
┌─────────────────────┐        D-Bus (System Bus)       ┌──────────────────────────┐
│  memreduct-gui      │ ──────────────────────────────▶ │  memreductd (daemon)     │
│  (User session,     │   Method: Reduce(mode)          │  (Root, systemd service) │
│   GTK4 / libadwaita │ ◀────────────────────────────── │  Guarded by Polkit       │
│   + StatusNotifier) │   Returns: JSON result          └──────────────────────────┘
└─────────────────────┘                                                │
        │                                                              │
        └── Direct unprivileged reads (/proc/meminfo, PSI, zram) ──────┘
```

---

### 3. Reduction Modes

| Mode | Target Mechanism | Safety Level | Description |
|---|---|---|---|
| `reclaim` (Default) | cgroup v2 `memory.reclaim` + `sync` | 🟢 Very Safe | Trims reclaimable anonymous/file pages cleanly |
| `pagecache` | `echo 1 > /proc/sys/vm/drop_caches` | 🟡 Moderate | Flushes & drops clean filesystem page cache |
| `slab` | `echo 2 > /proc/sys/vm/drop_caches` | 🟡 Moderate | Drops directory entries & inode slab caches |
| `all` | `echo 3 > /proc/sys/vm/drop_caches` | 🔴 Aggressive | Drops page cache, dentries, and inodes |
| `compact` | `echo 1 > /proc/sys/vm/compact_memory` | 🟢 Safe | Compacts kernel memory pages without dropping cache |

---

### 4. Configuration

Configuration file is automatically loaded and saved at:
`~/.config/memreduct/config.toml`

```toml
refresh_interval_secs = 1
start_minimized = false
show_tray = true
show_reduct_confirmation = true
tray_warning_level = 75.0
tray_danger_level = 90.0

[auto_reduce]
enabled_threshold = false
threshold_used_percent = 85.0
enabled_psi = false
psi_threshold_avg10 = 25.0
enabled_interval = false
interval_seconds = 1800
cooldown_seconds = 300
default_mode = "reclaim"
```

---

## 📄 License & Attribution

- Released under the **GNU General Public License v3.0 or later** ([GPL-3.0-or-later](LICENSE)).
- Concept and memory categorization inspired by [Mem Reduct](https://github.com/henrypp/memreduct) by Henry++.
- See [NOTICE](NOTICE) for full attribution.
