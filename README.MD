# XPlane UIPC

FSUIPC-like interface for XPlane.

Goals:

1. management of offsets in a config file;
2. stability - should not crash when changing aircraft or liveriles;
3. should work with Self Loading Cargo + ZiboMod 737

Non-goals:

1. supporting in-process access;
2. supporting WideFS UDP protocol;

## Building

```shell
cargo xtask build
cargo xtask dist
```

## License

GNU LGPLv3.

FSUIPC is a trademark of Pete Dowson. This plugin is an
independent implementation and is not affiliated with or endorsed by the
FSUIPC project.

XPlane SDK License [viewable here](./vendor/x-plane-sdk/4.3.0/SDK/license.txt).


# parking lot

Bindgen [requirements](https://rust-lang.github.io/rust-bindgen/requirements.html):
```shell
winget install LLVM.LLVM
```

Test with:
`C:\Users\chris\src\fsuipc-rs>cargo run --example hello` 
or SLC