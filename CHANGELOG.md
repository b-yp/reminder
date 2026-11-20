# Changelog

## 0.1.0 (2026-11-20)


### Features

* add empty state, text truncation and disabled state for mini capsule ([4fce9a5](https://github.com/b-yp/reminder/commit/4fce9a5bc99cc7fc90b26cc51dc2b97d73c5cdf4))
* collapse row actions into menu and add undo toast for deletion ([df47163](https://github.com/b-yp/reminder/commit/df471633671beaa5df675b01a73494e3b1d5415c))
* enhance button hit areas, add tooltips and toast progress bar ([7de138e](https://github.com/b-yp/reminder/commit/7de138e3ed8ee02b1ca64b37efa93f534bff0a4d))
* hide on close and add tray context menu ([f865b97](https://github.com/b-yp/reminder/commit/f865b978bd449812625579c502d382503a8fdea9))
* implement desktop floating widget for health and routine reminders ([f0e16fb](https://github.com/b-yp/reminder/commit/f0e16fbfcd1b75807d63b343d51eee5fd94d02b4))
* optimize recommended presets with collapsible and symmetric layout ([dcaa2fe](https://github.com/b-yp/reminder/commit/dcaa2fee965a7e3a74cce90ce8712b029d8a3e59))
* show today's check-in counts from per-day records ([16ce473](https://github.com/b-yp/reminder/commit/16ce473bb16a98f80ede3f64dbea01cc62756cd2))
* support creating custom reminders ([4c7db71](https://github.com/b-yp/reminder/commit/4c7db718624f7d5735a1b02bf7897da782824237))
* support fixed-time daily reminders ([98458f8](https://github.com/b-yp/reminder/commit/98458f80421d60662550e5fcc277e4962d5cf237))
* support frameless floating window on macOS ([188c68c](https://github.com/b-yp/reminder/commit/188c68c8f9d72957448272d47220eafd53595576))
* support global hotkey to hide and show the widget ([fe70a40](https://github.com/b-yp/reminder/commit/fe70a40ef4286531da3ea2fffa3d70122e1059d6))
* support launch at login ([158bfd8](https://github.com/b-yp/reminder/commit/158bfd8c094edda59dfb51b6c585a14df3a1294f))
* support light and dark theme modes ([cfb6db4](https://github.com/b-yp/reminder/commit/cfb6db42253ab74448eec0271e56ce5482dca672))
* support local data persistence ([6022da3](https://github.com/b-yp/reminder/commit/6022da392c642dc5e4a12d1f24330ff409f53d20))
* support mini capsule mode ([0e6f2ac](https://github.com/b-yp/reminder/commit/0e6f2ac60b7e70f47e825b45f6554b34cf9755aa))
* support rounded app logo with taskbar and system tray integration ([651e4e7](https://github.com/b-yp/reminder/commit/651e4e70f2fd826ed3440e2fd9839931c88e2e4a))
* support window opacity adjustment ([edde2fa](https://github.com/b-yp/reminder/commit/edde2fafd4e694962d011c6eec4ede2e0f00b0ee))


### Bug Fixes

* **ci:** install gstreamer dev packages for the Linux build ([922caac](https://github.com/b-yp/reminder/commit/922caac61e3a7ee6aa82b75bd2e5b1b926aec18b))
* **ci:** pass --repo to gh workflow run ([ea40316](https://github.com/b-yp/reminder/commit/ea403164ab10720bd212f849eb4394e3ba1eb254))
* insert new reminders at top and restore undo by id ([4940d98](https://github.com/b-yp/reminder/commit/4940d9860216c7647d6f818f23b061a8928c01b4))
* **localtime:** use nsh_platform_windows for the Windows time API ([2d4b936](https://github.com/b-yp/reminder/commit/2d4b9366f237f6c32d6243ee6c74ee8c99667aa2))
* make the macOS window transparent for rounded corners ([712f103](https://github.com/b-yp/reminder/commit/712f103fdddfb797eac2dc19d67e75458d1f1c73))
* prevent long item name from overflowing reminder row ([ecf8c89](https://github.com/b-yp/reminder/commit/ecf8c891da125cc58be554f0b3dfb57b39a76fe4))
* remove redundant root border and rounded corners on Windows to avoid ghosting ([eec362f](https://github.com/b-yp/reminder/commit/eec362f5fa1809b003dfde92bdbaff732151f7cd))
* **ui:** improve light theme contrast and typography ([5585a54](https://github.com/b-yp/reminder/commit/5585a549afbc27520685f0a1916c199b8776c008))
* **ui:** use button as row menu trigger ([5171159](https://github.com/b-yp/reminder/commit/5171159bde00fb6bf6eefb49bc1f823058c1d21e))
* unify app id and version ([2f1bf54](https://github.com/b-yp/reminder/commit/2f1bf54cb5c395a811e7b8850710d676014e26de))
