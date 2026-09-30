# glint

A small system fetch for Linux. No dependencies, no subprocesses. It reads
everything from `/proc`, `/sys` and `/etc`.

```
                           tbolt@ghost
                 .-.       ───────────
          .----.(   )       os Ubuntu 24.04.5 LTS x86_64
        .'      '-'`.       host ASUS ROG STRIX B650E-I GAMING WIFI
       /              \     kernel 7.0.0-31-generic
   .-.|                |    uptime 100years
  (   )                |    pkgs 2108 dpkg, 16 snap
   '-'|                |    shell bash
       \              /     cpu AMD Ryzen 7 7800X3D (16) @ 5.02GHz
        '.      .-.,'       gpu NVIDIA GeForce RTX 4070
          '----(   )        gpu AMD Raphael (iGPU)
                '-'         mem 3.3G / 30.5G  ━───────── 11%
                            disk 658.9G / 914.8G  ━━━━━━━─── 72%
                             
```

## Install

```sh
cargo install --path .
```

## Usage

```
glint [--no-color] [--no-logo]
```

Colour is also turned off when `NO_COLOR` is set.

Logos: Ubuntu, Arch including EndeavourOS, Manjaro and CachyOS, Debian including
Raspbian, Fedora. Anything else gets Tux.

Needs a 64-bit target.
