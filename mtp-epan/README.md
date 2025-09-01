# Wireshark MTP Dissector

## Build Instructions

```shell
$ cd mtp-epan
$ mkdir build && cd build
$ cmake -DCMAKE_INSTALL_PREFIX=/usr ..
$ make
```

### Installing

Note this will install both the Rust library and the Wireshark plugin system-wide.

```shell
$ sudo make install
```

## Usage

Verify the plugin is loaded by going to `Help > About Wireshark > Plugins` and searching for `mtp`.

MTP traffic can be filtered with `_ws.col.protocol == "MTP"`
