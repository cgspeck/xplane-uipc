# List of Self-Loading-Cargo Offsets

## References

### FSUIPC Offsets

[Project Magenta Downloads page](https://www.projectmagenta.com/downloads/) - specifically [FSUIPC Offsets Quick Reference](https://www.projectmagenta.com/download/8733/)

[All About FSUIPC Page](https://fsuipc.com/about/) - search for "documentation" for [stand-alone download pack](https://fsuipc.com/download/FSUIPC7_Documentation.zip).

[FSUIPC Downloads Page](https://fsuipc.com/): Download FSUIPC7 then look at `changes.txt`.

[SquawkBox 3 User Manual](https://squawkbox.ca/doc/sdk/fsuipc.ph)

[FSUIPC 7.5.6d beta announcement (0x0272 and 0x0273)](https://fsuipc.com/new-fsuipc7-beta-version-available-7-5-6d-including-wasm-wapi-v1-1-0-and-with-full-installer/)

### X-Plane Datarefs

[X-Plane Datarefs](https://developer.x-plane.com/datarefs/)

[DataRefTool for X-Plane](https://datareftool.com/) to verify and test DataRefs in the sim.

## Supported

### Equivalent Datarefs

```
# offset, n_bytes
0x11ba, 2
0x3098, 8
0x30a0, 8
0x3090, 8
0x0bb2, 2
0x0bb6, 2
0x0bba, 2
0x3367, 1
0x0b52, 1
0x0b5c, 4
0x0b54, 4
0x0354, 2
0x0c1a, 2
0x0e8c, 2
0x0264, 2
0x02bc, 4
0x6010, 8
0x6018, 8
0x3324, 4
0x0580, 4
0x0bd0, 4
0x2e80, 4
0x281c, 4
0x3102, 2
0x0366, 2
0x02c8, 4
0x0d0c, 2
0x02b4, 4
0x2f70, 8
0x2f78, 8
0x30a8, 8
0x30b0, 8
0x30b8, 8
0x3060, 8
0x3068, 8
0x3070, 8
0x0330, 2
0x0aec, 4
0x0898, 4
0x0930, 4
0x09c8, 4
0x0a60, 4
0x126c, 4
0x0bc8, 4
0x341d, 1
0x0840, 2
0x028c, 1
0x0bdc, 4
0x0be8, 4
0x0238, 1
0x0239, 1
0x023a, 1
0x023b, 1
0x023c, 1
0x115e, 1
0x0568, 8
0x0560, 8
0x0578, 4
0x057c, 4
0x0570, 8
0x7b91, 1
0x023e, 2
0x030c, 4
```

### Derived

```
# offset, n_bytes
0x3367, 1  # various exit doors as a bitmask - note: no documentation about which door corresponds with which bit
0x31F0, 4  - pushback status 3=off, 0=pushing back, 1=pushing back, tail to swing to left (port), 2=pushing back, tail to swing to right (starboard). Only 3 and 0 supported.
```

### Supplied by helper LUA script

```
0x0246, 2  # negated UTC offset in minutes (i.e. +10 hours is -600)
# local date - need injection
0x0245, 1  # local day of month
0x0244, 1  # local month of year
0x024a, 2  # local year
# zulu date - need injection
0x0240, 2  # zulu year
0x023d, 1  # zulu day of month
0x0242, 1  # zulu month of year
```

### Statically Faked

Change these inside your installation as required.

```
# offset, n_bytes
0x3364, 1  # ready to fly
0x3365, 1  # in menu
0x3500, 24  # ATC model name
0x3d00, 256  # Aircraft title
0x313c, 12  # ATC ID / Tail Number
0x3160, 24  # ATC type
0x3148, 24  # ATC Airline
```

## Unsupported

### No XPlane equivalent

The following offsets do not appear to have a directly equivalent dataref:

```
# offset, n_bytes
0x290c, 4  # number of hot joystick button slots avaliable
0x3126, 1  # set view mode
0x31e8, 4  # surface type, does not map
0x0c18, 2  # units of measure?? 0 = us, 1 = metric + feet, 2 = metric + meters
0x0658, 4 # docs says its 120 bytes with details about 6 nearest airports
0x8320, 1 # docs say "view mode" and "not working"
0x3c00, 256 # path to current air file
0x31f4, 4  # set pushback status
0x0272, 2  # on any runway
0x0273, 2  # ambient in cloud
```