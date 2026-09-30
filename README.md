# Mini-altimètre

Deux versions de l'altimètre, une en python dans `./py` et une en rust dans `./rs`.

- Version en python dans `./py`.
  - Dépendences: `python` et `mpremote` (s'installe avec `pipx` ou `pacman`) ou de l'extension VSCode.
  - Run: `mpremote device /dev/ttyACM0 blink.py`
- Version en rust dans `./rs`.
  - Dépendences: `rust`, `elf2uf2-rs` (`cargo binstall elf2uf2-rs`) et être sous POSIX.
  - Run: `cargo run --release`
