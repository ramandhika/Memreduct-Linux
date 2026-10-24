# PROJECT: `memreduct-linux` — Real-time Memory Monitor & Optimizer untuk CachyOS

> File ini adalah **prompt induk / spesifikasi proyek** untuk AI coding agent (Claude Code, dll).
> Baca seluruh file sebelum menulis kode. Ikuti urutan fase di bagian 9.

---

## 1. Tujuan

Bangun aplikasi desktop Linux native, bernama sementara **`memreduct-linux`**, yang terinspirasi dari [Mem Reduct (henrypp/memreduct)](https://github.com/henrypp/memreduct) di Windows:

- Memantau penggunaan memori sistem **secara real-time** (RAM, swap/zram, cache, buffer, dll).
- Menyediakan aksi **"Reduce / Clean memory"** manual dan otomatis (berdasarkan ambang batas dan/atau interval).
- Berjalan di **system tray**, ringan, dan bisa diinstal di **CachyOS** (Arch-based) lewat `PKGBUILD` / paket `pacman`.

**Target pengguna:** pengguna CachyOS (KDE Plasma sebagai default utama, tapi harus tetap berjalan baik di GNOME, Hyprland, dll).

---

## 2. Tool / MCP yang WAJIB digunakan

Gunakan tool berikut selama pengerjaan. Jangan menebak API atau perilaku jika bisa diverifikasi lewat tool ini.

### 2.1 MCP "Reverse Engineer Anything"
Gunakan untuk **memahami cara kerja Mem Reduct asli** sebelum mendesain fitur Linux-nya:

1. Analisis repo `henrypp/memreduct`: struktur modul, alur kerja monitor, daftar "memory region" yang dibersihkan (working set, system file cache, standby list, modified page list, dll), pengaturan (settings), mekanisme auto-reduce (threshold %, interval menit), tray icon, dan hotkey.
2. Hasilkan **dokumen pemetaan fitur** `docs/feature-mapping.md`: tabel *fitur Windows → padanan Linux → catatan/risiko*.
3. **Jangan menyalin kode** secara langsung. Mem Reduct berlisensi GPL-3.0; kita hanya mempelajari perilaku dan konsepnya. Tulis ulang implementasi dari nol. Jika ada bagian yang diturunkan langsung, proyek ini harus dirilis GPL-3.0-or-later dan itu wajib dicatat di `NOTICE`.

### 2.2 Context7
Gunakan **Context7** untuk mengambil dokumentasi **terbaru dan sesuai versi** setiap kali memakai library/framework. Wajib dicek sebelum menulis kode yang memakai:

- `gtk4-rs`, `libadwaita-rs` (atau Qt/`cxx-qt` bila dipilih)
- `zbus` (D-Bus), `polkit`
- `tokio`, `serde`, `clap`
- `sysinfo` / `procfs`
- `ksni` (StatusNotifierItem / tray)
- `tracing`, `thiserror`
- Dokumentasi kernel terkait: `/proc/meminfo`, `/proc/pressure/memory` (PSI), `vm.drop_caches`, `vm.compact_memory`, cgroup v2 `memory.reclaim`
- Packaging: `PKGBUILD`, `makepkg`, systemd unit

Aturan: tulis `use context7` pada prompt ke tool tersebut, catat versi library yang dipakai di `docs/dependencies.md`.

### 2.3 Ponytail
> ⚠️ **TODO untuk pemilik proyek:** isi bagian ini sesuai fungsi "ponytail" yang kamu maksud (mis. MCP/tool/agent tertentu). Saya tidak ingin menebak fungsinya.

Template yang bisa dipakai:

```
Ponytail dipakai untuk: <ISI: mis. orkestrasi task / code review / testing / dll>
Kapan dipanggil:        <ISI: mis. setelah setiap fase selesai>
Output yang diharapkan: <ISI>
```

Jika Ponytail tidak tersedia saat dijalankan, lanjutkan pekerjaan tanpa berhenti dan catat di `docs/decisions.md` bahwa langkah tersebut dilewati.

---

## 3. Realitas Teknis (PENTING — baca sebelum mendesain)

Linux **tidak sama dengan Windows**. Kernel Linux sudah memakai RAM kosong sebagai page cache secara agresif dan melepasnya otomatis saat dibutuhkan. Konsekuensinya:

- "RAM terpakai tinggi" di Linux **biasanya normal** (sebagian besar hanya cache).
- Menjalankan `drop_caches` terlalu sering justru **memperlambat** sistem karena cache harus dibaca ulang dari disk.
- Maka aplikasi ini harus **monitor-first**. Aksi pembersihan bersifat **opsional**, punya peringatan jelas di UI, dan **default-nya konservatif**.
- Tampilkan metrik yang jujur: gunakan `MemAvailable` (bukan `MemFree`) sebagai "memori yang benar-benar tersedia", dan tampilkan **PSI memory pressure** sebagai indikator tekanan memori sebenarnya.
- CachyOS secara default memakai **zram**, jadi tampilkan statistik zram (ukuran terkompresi vs asli, rasio kompresi) terpisah dari swap disk.

---

## 4. Pemetaan Fitur (Windows → Linux)

| Mem Reduct (Windows) | Padanan Linux | Hak akses | Catatan |
|---|---|---|---|
| Monitor RAM / pagefile | `/proc/meminfo`, `/proc/swaps`, `/sys/block/zram*/` | user biasa | Refresh 1 detik (konfigurabel) |
| Reduce *System file cache* | `sync` + `echo 1 > /proc/sys/vm/drop_caches` (pagecache) | root | Opsi "Clean cache" |
| Reduce *Slab/dentry/inode* | `echo 2 > /proc/sys/vm/drop_caches` | root | Opsional, tingkat lanjut |
| Reduce *Working set* | cgroup v2 `memory.reclaim` per cgroup/proses user | user/root | Lebih aman daripada drop_caches; prioritaskan ini |
| Reduce *Modified/Standby list* | `sync` (flush dirty pages) | user | Aman |
| Defrag/compact | `echo 1 > /proc/sys/vm/compact_memory` | root | Opsional |
| Reduksi swap | **Jangan otomatis** `swapoff/swapon`. Tampilkan saja, beri opsi manual dengan cek RAM tersedia | root | Berisiko OOM bila RAM tidak cukup |
| Auto-reduce by threshold | Timer + ambang `MemAvailable %` atau PSI `some avg10` | — | Dengan cooldown minimum |
| Tray icon + tooltip | StatusNotifierItem (D-Bus SNI) | user | Kompatibel KDE/GNOME (butuh ekstensi AppIndicator) |
| Hotkey global | Via `.desktop` action / D-Bus method (hotkey diatur oleh DE) | user | Dokumentasikan cara binding |

Tindakan berhak akses root **tidak boleh** dijalankan oleh GUI. Lihat arsitektur di bagian 5.

---

## 5. Arsitektur

Pisahkan hak akses (*privilege separation*):

```
┌─────────────────────┐        D-Bus (system bus)        ┌──────────────────────────┐
│  memreduct-gui      │ ───────────────────────────────▶ │  memreductd (daemon)     │
│  (user, GTK4/Adwaita│   method: Reduce(mode)           │  systemd service, root   │
│   + tray SNI)       │ ◀─────────────────────────────── │  Polkit-authorized       │
└─────────────────────┘   signal: ReduceFinished(stats)  └──────────────────────────┘
        │                                                          │
        └── baca langsung /proc/meminfo, PSI (tanpa root) ─────────┘
```

**Komponen:**

1. **`memreduct-core`** (library Rust): parsing `/proc/meminfo`, PSI, zram, serta logika kebijakan auto-reduce. Tanpa dependensi GUI sehingga mudah diuji.
2. **`memreductd`** (daemon root, aktivasi via D-Bus/systemd): hanya mengekspos metode terbatas (`Reduce(mode)` dengan *whitelist* mode: `pagecache`, `slab`, `compact`, `reclaim`). Setiap pemanggilan diotorisasi via **Polkit** (`org.memreduct.reduce`). Rate-limit + validasi input ketat.
3. **`memreduct-gui`** (user): jendela utama, tray, pengaturan, notifikasi.
4. **`memreduct-cli`** (opsional tapi disarankan): `memreduct status`, `memreduct reduce --mode pagecache`, output JSON untuk scripting.

**Pilihan stack (default, bisa diubah dengan alasan tertulis di `docs/decisions.md`):**

- Bahasa: **Rust** (stabil, rendah footprint, aman memori)
- GUI: **GTK4 + libadwaita** (alternatif: Qt6 bila dirasa lebih cocok dengan KDE default CachyOS; putuskan di Fase 1 dan catat alasannya)
- IPC: **zbus**
- Konfigurasi: TOML di `~/.config/memreduct/config.toml`
- Logging: `tracing` ke journald

---

## 6. Fitur Fungsional

### 6.1 Monitor
- Gauge/bar: RAM terpakai, tersedia, cache/buffer, swap, zram.
- Grafik riwayat (mis. 60 detik – 10 menit) ringan, tanpa library berat.
- Indikator PSI memory pressure (hijau / kuning / merah).
- Daftar top proses berdasarkan RSS/PSS (opsional, dibaca via `/proc`).
- Tooltip tray: `Tersedia X GB (Y%)`; ikon berubah warna sesuai level.

### 6.2 Reduce
- Tombol **Reduce Now** dengan pilihan mode (aman → agresif).
- Tampilkan **hasil sebelum/sesudah** (MB dibebaskan) setelah selesai.
- Dialog konfirmasi pertama kali untuk mode agresif, berisi penjelasan efek sampingnya.

### 6.3 Auto-reduce
- Pemicu: ambang `MemAvailable` (%), atau PSI, atau interval waktu.
- **Cooldown minimum** (default 5 menit) agar tidak terjadi *thrashing*.
- Default: **dimatikan**. Mode default bila diaktifkan: `reclaim` / `sync` (yang paling aman).

### 6.4 Pengaturan
- Interval refresh, tema (ikut sistem), mulai otomatis saat login (XDG autostart), tampilkan/sembunyikan tray, mode reduce default, ambang batas, notifikasi on/off.

### 6.5 Integrasi sistem
- Berkas `.desktop` + ikon (SVG) mengikuti spesifikasi freedesktop.
- Mendukung Wayland dan X11.
- Terjemahan: Bahasa Indonesia dan Inggris (gettext/`fluent`).

---

## 7. Persyaratan Non-Fungsional

- **Ringan:** CPU idle < 1%, memori GUI < ~60 MB saat berjalan di tray.
- **Aman:** tidak ada eksekusi shell string; tulis ke `/proc/sys/vm/*` langsung dari daemon dengan path *hardcoded*; tidak ada jalur privilege escalation lewat input GUI.
- **Tahan gagal:** jika daemon tidak ada, GUI tetap berfungsi sebagai monitor dan menampilkan status "aksi reduce tidak tersedia".
- **Dapat diuji:** unit test untuk parser dan kebijakan; test integrasi dengan fixture `/proc/meminfo` palsu.
- **Lisensi:** tentukan di Fase 0 (GPL-3.0-or-later disarankan bila ada keterkaitan dengan Mem Reduct; selain itu MIT/Apache-2.0 boleh).

---

## 8. Struktur Repo

```
memreduct-linux/
├── Cargo.toml                  # workspace
├── crates/
│   ├── memreduct-core/
│   ├── memreductd/
│   ├── memreduct-gui/
│   └── memreduct-cli/
├── data/
│   ├── org.memreduct.Daemon.service        # D-Bus activation
│   ├── org.memreduct.policy                # Polkit
│   ├── memreductd.service                  # systemd
│   ├── memreduct.desktop
│   └── icons/
├── packaging/
│   └── arch/PKGBUILD
├── docs/
│   ├── feature-mapping.md
│   ├── dependencies.md
│   └── decisions.md
├── NOTICE
├── LICENSE
└── README.md
```

---

## 9. Rencana Kerja (Fase)

Selesaikan satu fase sebelum lanjut. Di akhir tiap fase: jalankan `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, lalu ringkas hasil (dan panggil Ponytail bila sudah dikonfigurasi).

**Fase 0 — Riset (Reverse Engineer + Context7)**
- Analisis `henrypp/memreduct` → `docs/feature-mapping.md`.
- Verifikasi API kernel & library lewat Context7 → `docs/dependencies.md`.
- Tentukan lisensi, GUI toolkit final → `docs/decisions.md`.

**Fase 1 — Core**
- Parser `/proc/meminfo`, PSI, zram; struct `MemorySnapshot`; unit test.

**Fase 2 — Daemon & Polkit**
- `memreductd` dengan D-Bus, mode whitelist, otorisasi Polkit, rate-limit.

**Fase 3 — CLI**
- `memreduct status [--json]`, `memreduct reduce --mode ...`.

**Fase 4 — GUI & Tray**
- Jendela utama, grafik, tray SNI, notifikasi, halaman pengaturan.

**Fase 5 — Auto-reduce & Konfigurasi**
- Mesin kebijakan (threshold/PSI/interval + cooldown), simpan konfigurasi.

**Fase 6 — Packaging CachyOS/Arch**
- `PKGBUILD` yang menginstal binary, unit systemd, file D-Bus, Polkit, `.desktop`, ikon.
- Uji: `makepkg -si` di CachyOS bersih (atau container Arch).

**Fase 7 — Polish**
- i18n (id/en), dokumentasi README (instalasi, penggunaan, peringatan soal drop_caches), screenshot, tangkapan lisensi.

---

## 10. Definition of Done

- [ ] `makepkg -si` berhasil di CachyOS dan aplikasi muncul di menu aplikasi.
- [ ] Tray menampilkan data real-time yang cocok dengan output `free -h`.
- [ ] "Reduce Now" meminta otorisasi Polkit dan menampilkan hasil sebelum/sesudah.
- [ ] Auto-reduce nonaktif secara default, dan cooldown berfungsi.
- [ ] GUI tetap berjalan sebagai monitor saat daemon tidak aktif.
- [ ] Semua test hijau; tidak ada warning clippy.
- [ ] README menjelaskan dengan jujur kapan reduce berguna dan kapan tidak.

---

## 11. Aturan untuk Agent

1. **Verifikasi dulu, tulis kemudian:** pakai Context7 sebelum memakai API library mana pun.
2. **Jangan menyalin kode Mem Reduct**; pakai hanya sebagai referensi perilaku.
3. **Jangan pernah** menjalankan perintah root dari GUI; semua lewat daemon + Polkit.
4. Jika ada keputusan desain yang ambigu, pilih opsi paling **aman dan sederhana**, lalu catat di `docs/decisions.md`.
5. Jangan menambah fitur di luar dokumen ini tanpa konfirmasi.
6. Setiap fase berakhir dengan ringkasan: apa yang selesai, apa yang diuji, apa yang tersisa.

---

## 12. Prompt Pembuka (tempel ke agent)

```
Baca project.md di repo ini sepenuhnya. Mulai dari Fase 0:
1) Gunakan MCP Reverse Engineer Anything untuk menganalisis https://github.com/henrypp/memreduct
   dan buat docs/feature-mapping.md (Windows -> Linux).
2) Gunakan Context7 untuk memverifikasi dokumentasi terbaru library dan interface kernel
   yang tercantum di bagian 2.2.
3) Buat docs/decisions.md berisi pilihan lisensi dan GUI toolkit beserta alasannya.
Berhenti setelah Fase 0 dan tunjukkan ringkasan sebelum lanjut ke Fase 1.
```
