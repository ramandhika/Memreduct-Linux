# Architectural & Design Decisions

Catatan keputusan arsitektur (ADR) untuk proyek `memreduct-linux`.

---

## Keputusan 1: Pilihan Lisensi

- **Keputusan:** Menggunakan lisensi **GPL-3.0-or-later**.
- **Alasan:**
  1. Perangkat lunak referensi ([henrypp/memreduct](https://github.com/henrypp/memreduct)) dilisensikan dengan **GPL-3.0**. Meskipun implementasi Linux ini ditulis ulang 100% dari nol dalam Rust tanpa menyalin satu baris pun dari kode sumber C asli, konsep fungsi pembersihan, penamaan, dan pembagian memori terinspirasi langsung dari Mem Reduct.
  2. Lisensi GPL-3.0-or-later menjamin kepatuhan hak cipta penuh, kebebasan perangkat lunak sumber terbuka bagi komunitas CachyOS/Arch, serta perlindungan terhadap derivasi tertutup (*anti-tivoization*).
  3. Dokumen `NOTICE` mencantumkan atribusi kepada proyek asli Henry++ Mem Reduct.

---

## Keputusan 2: Pilihan GUI Toolkit

- **Keputusan:** Menggunakan **GTK4 + libadwaita** dipadukan dengan **`ksni`** untuk integrasi System Tray.
- **Alternatif yang Dipertimbangkan:**
  - *Qt6 via C++ / QML*: Sangat umum di KDE Plasma, tetapi memerlukan rantai kompilasi C++ ganda (`cxx-qt` atau bindgen), tooling CMake tambahan, serta jejak dependensi yang lebih rumit untuk workspace Cargo murni.
  - *Slint / Iced*: Ringan, namun integrasi tray dan dialog sistem Linux masih memerlukan binding manual tambahan.
- **Alasan Pemilihan GTK4 + libadwaita:**
  1. **Dukungan Rust Kelas Satu:** Rust bindings (`gtk4-rs`, `libadwaita-rs`) didukung penuh, aman, dan dapat dikompilasi secara deterministik via `cargo`.
  2. **Dukungan Wayland & Flatpak/Arch Native:** Berjalan lancar di Wayland (KDE Plasma, GNOME, Hyprland, Sway) tanpa glitch rendering.
  3. **Tema Adaptif:** Otomatis mengikuti preferensi dark/light mode desktop KDE Plasma melalui standar XDG `org.freedesktop.appearance.color-scheme`.
  4. **System Tray Mandiri via StatusNotifierItem (`ksni`):** Pada desktop KDE Plasma (default CachyOS), tray icon menggunakan protokol D-Bus StatusNotifierItem standar freedesktop. Crate `ksni` mengimplementasikan protokol ini secara native dalam Rust, sehingga tray icon terintegrasi sempurna di panel KDE Plasma tanpa bergantung pada plugin GTK tray lama.

---

## Keputusan 3: Penanganan Modul Ponytail

- **Status:** Tool/MCP Ponytail tidak didefinisikan secara khusus pada instruksi proyek saat ini selain aturan umum harness.
- **Tindakan:** Berdasarkan bagian 2.3 `project.md`, pengerjaan tetap dilanjutkan tanpa hambatan dan dicatat di sini.
