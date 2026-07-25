# Third-Party Notices

idevice desktop is distributed under the MIT License and links third-party
open-source software. This inventory is generated from the resolved
dependency graph for `aarch64-apple-darwin`, the only platform the project
builds for, so it reflects what a release actually ships rather than every
optional dependency in the lock file.

- Rust crates linked into the binary: 371
- npm packages bundled into the frontend: 9
- npm packages used only to build, and therefore not distributed: 70

No dependency is licensed under the GPL, LGPL, or AGPL, so none of them
places a copyleft obligation on this project.


## Weak-copyleft dependencies

These are licensed under the Mozilla Public License 2.0, a file-level
copyleft. Using them as unmodified dependencies imposes no obligation on
this project, but modifying their source would require publishing those
modifications. This project does not modify them.

- cssparser 0.36.0
- cssparser-macros 0.6.1
- dtoa-short 0.3.5
- option-ext 0.2.0
- selectors 0.36.1

Their sources are available at <https://crates.io>.


## License distribution

| License | Rust crates | npm packages |
| --- | --- | --- |
| MIT OR Apache-2.0 | 166 | 1 |
| MIT | 72 | 5 |
| Apache-2.0 OR MIT | 54 | 1 |
| Unicode-3.0 | 18 |  |
| MIT/Apache-2.0 | 15 |  |
| Zlib OR Apache-2.0 OR MIT | 8 |  |
| BSD-3-Clause | 6 |  |
| MPL-2.0 | 5 |  |
| Apache-2.0 | 4 |  |
| ISC | 2 | 1 |
| Unlicense OR MIT | 3 |  |
| MIT OR Apache-2.0 OR Zlib | 2 |  |
| Unlicense/MIT | 2 |  |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 |  |
| 0BSD OR MIT OR Apache-2.0 | 1 |  |
| Apache-2.0 / MIT | 1 |  |
| Apache-2.0 AND ISC | 1 |  |
| Apache-2.0 AND MIT | 1 |  |
| Apache-2.0 OR ISC OR MIT | 1 |  |
| Apache-2.0/MIT | 1 |  |
| BSD-2-Clause |  | 1 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 1 |  |
| BSD-3-Clause AND MIT | 1 |  |
| BSD-3-Clause/MIT | 1 |  |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |  |
| MIT OR BSD-3-Clause | 1 |  |
| MIT OR Zlib OR Apache-2.0 | 1 |  |
| Zlib | 1 |  |

## Primary dependency

## idevice

- Project: `jkcoxson/idevice`
- Source: https://github.com/jkcoxson/idevice
- Pinned revision: `8eed181f39a16ea70380ec8c3cff6bed07a1ef69`
- License: MIT

Copyright 2026 Jackson Coxson

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the “Software”), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


## Complete inventory

Each dependency is listed with the license its own metadata declares. Full
license texts ship with each package and are available from
<https://crates.io> and <https://www.npmjs.com>.


### Rust crates


**(MIT OR Apache-2.0) AND Unicode-3.0** (1)

- unicode-ident 1.0.24

**0BSD OR MIT OR Apache-2.0** (1)

- adler2 2.0.1

**Apache-2.0** (4)

- flagset 0.4.7
- sync_wrapper 1.0.2
- tao 0.35.3
- zopfli 0.8.3

**Apache-2.0 / MIT** (1)

- fnv 1.0.7

**Apache-2.0 AND ISC** (1)

- ring 0.17.14

**Apache-2.0 AND MIT** (1)

- dpi 0.1.2

**Apache-2.0 OR ISC OR MIT** (1)

- rustls 0.23.42

**Apache-2.0 OR MIT** (54)

- atomic-waker 1.1.2
- autocfg 1.5.1
- base64ct 1.8.3
- bit-set 0.8.0
- bit-vec 0.8.0
- cargo_toml 0.22.3
- chacha20 0.9.1
- chacha20poly1305 0.10.1
- const-oid 0.9.6
- ctor 0.8.0
- ctor-proc-macro 0.0.7
- der 0.7.10
- der_derive 0.7.3
- dtor 0.3.0
- dtor-proc-macro 0.0.6
- ed25519 2.2.3
- equivalent 1.0.2
- fastrand 2.4.1
- futures-lite 2.6.1
- idna_adapter 1.2.2
- indexmap 1.9.3
- indexmap 2.14.0
- mdns-sd 0.19.2
- muda 0.19.3
- parking 2.2.1
- pem-rfc7468 0.7.0
- pin-project 1.1.13
- pin-project-internal 1.1.13
- pin-project-lite 0.2.17
- pkcs1 0.7.5
- pkcs8 0.10.2
- poly1305 0.8.0
- rustc-hash 2.1.3
- signature 2.2.0
- spki 0.7.3
- tauri 2.11.5
- tauri-build 2.6.3
- tauri-codegen 2.6.3
- tauri-macros 2.6.3
- tauri-plugin 2.6.3
- tauri-plugin-dialog 2.7.1
- tauri-plugin-fs 2.5.1
- tauri-runtime 2.11.3
- tauri-runtime-wry 2.11.4
- tauri-utils 2.9.3
- tls_codec 0.4.2
- tls_codec_derive 0.4.2
- utf8_iter 1.0.4
- uuid 1.24.0
- window-vibrancy 0.6.0
- wry 0.55.1
- x509-cert 0.2.5
- zeroize 1.9.0
- zeroize_derive 1.5.0

**Apache-2.0/MIT** (1)

- flume 0.11.1

**BSD-2-Clause OR Apache-2.0 OR MIT** (1)

- zerocopy 0.8.54

**BSD-3-Clause** (6)

- alloc-no-stdlib 2.0.4
- alloc-stdlib 0.2.4
- curve25519-dalek 4.1.3
- ed25519-dalek 2.2.0
- subtle 2.6.1
- x25519-dalek 2.0.1

**BSD-3-Clause AND MIT** (1)

- brotli 8.0.4

**BSD-3-Clause/MIT** (1)

- brotli-decompressor 5.0.3

**CC0-1.0 OR MIT-0 OR Apache-2.0** (1)

- dunce 1.0.5

**ISC** (2)

- rustls-webpki 0.103.13
- untrusted 0.9.0

**MIT** (72)

- async-stream 0.3.6
- async-stream-impl 0.3.6
- async_zip 0.0.18
- block2 0.6.2
- bytes 1.12.1
- cargo_metadata 0.19.2
- cfb 0.7.3
- darling 0.23.0
- darling_core 0.23.0
- darling_macro 0.23.0
- derive_more 2.1.1
- derive_more-impl 2.1.1
- dom_query 0.27.0
- embed-resource 3.0.11
- generic-array 0.14.7
- http-body 1.1.0
- http-body-util 0.1.4
- hyper 1.10.1
- hyper-util 0.1.20
- ico 0.5.0
- idevice 0.1.65
- infer 0.19.0
- jktcp 0.1.6
- libm 0.2.16
- matchers 0.2.0
- mio 1.2.2
- new_debug_unreachable 1.0.6
- nu-ansi-term 0.50.3
- objc2 0.6.4
- objc2-encode 4.1.0
- objc2-foundation 0.3.2
- phf 0.13.1
- phf_codegen 0.13.1
- phf_generator 0.13.1
- phf_macros 0.13.1
- phf_shared 0.13.1
- plist 1.10.0
- plist-macro 0.1.6
- precomputed-hash 0.1.1
- quick-xml 0.41.0
- rfd 0.16.0
- schemars 0.8.22
- schemars 0.9.0
- schemars 1.2.1
- schemars_derive 0.8.22
- sharded-slab 0.1.7
- simd-adler32 0.3.10
- slab 0.4.12
- socket-pktinfo 0.3.2
- spin 0.9.9
- strsim 0.11.1
- synstructure 0.13.2
- tauri-winres 0.3.6
- tokio 1.52.4
- tokio-macros 2.7.0
- tokio-util 0.7.18
- tower 0.5.3
- tower-http 0.6.11
- tower-layer 0.3.3
- tower-service 0.3.3
- tracing 0.1.44
- tracing-attributes 0.1.31
- tracing-core 0.1.36
- tracing-log 0.2.0
- tracing-subscriber 0.3.23
- try-lock 0.2.5
- urlpattern 0.3.0
- want 0.3.1
- winnow 0.7.15
- winnow 1.0.4
- zip 2.4.2
- zmij 1.0.23

**MIT OR Apache-2.0** (166)

- aead 0.5.2
- aes 0.8.4
- anyhow 1.0.103
- async-compression 0.4.42
- base64 0.21.7
- base64 0.22.1
- bitflags 2.13.1
- block-buffer 0.10.4
- block-padding 0.3.3
- bumpalo 3.20.3
- camino 1.2.4
- cargo-platform 0.1.9
- cbc 0.1.2
- cc 1.2.67
- cfg-if 1.0.4
- chacha20 0.10.1
- chrono 0.4.45
- cipher 0.4.4
- compression-codecs 0.4.38
- compression-core 0.4.32
- cookie 0.18.1
- core-foundation 0.10.1
- core-foundation-sys 0.8.7
- core-graphics 0.25.0
- core-graphics-types 0.2.0
- cpufeatures 0.2.17
- crc32fast 1.5.0
- crossbeam-channel 0.5.16
- crossbeam-utils 0.8.22
- crypto-common 0.1.7
- deranged 0.5.8
- digest 0.10.7
- dirs 6.0.0
- dirs-sys 0.5.0
- displaydoc 0.2.6
- dtoa 1.0.11
- dyn-clone 1.0.20
- embed_plist 1.2.2
- erased-serde 0.4.10
- errno 0.3.14
- fdeflate 0.3.7
- find-msvc-tools 0.1.9
- flate2 1.1.9
- form_urlencoded 1.2.2
- futures 0.3.32
- futures-channel 0.3.32
- futures-core 0.3.32
- futures-executor 0.3.32
- futures-io 0.3.32
- futures-macro 0.3.32
- futures-sink 0.3.32
- futures-task 0.3.32
- futures-util 0.3.32
- getrandom 0.2.17
- getrandom 0.3.4
- getrandom 0.4.3
- glob 0.3.3
- hashbrown 0.12.3
- hashbrown 0.17.1
- heck 0.5.0
- hex 0.4.3
- hkdf 0.12.4
- hmac 0.12.1
- html5ever 0.38.0
- http 1.4.2
- httparse 1.10.1
- iana-time-zone 0.1.65
- idevice-srp 0.6.0
- idna 1.1.0
- inout 0.1.4
- ipnet 2.12.0
- itoa 1.0.18
- jsonptr 0.6.3
- keyboard-types 0.7.0
- lazy_static 1.5.0
- libc 0.2.186
- lock_api 0.4.14
- log 0.4.33
- markup5ever 0.38.0
- md-5 0.10.6
- mime 0.3.17
- ns-keyed-archive 0.1.5
- nskeyedarchiver_converter 0.1.3
- num-bigint 0.4.8
- num-conv 0.2.2
- num-integer 0.1.46
- num-iter 0.1.46
- num-traits 0.2.19
- once_cell 1.21.4
- opaque-debug 0.3.1
- parking_lot 0.12.5
- parking_lot_core 0.9.12
- percent-encoding 2.3.2
- png 0.17.16
- png 0.18.1
- powerfmt 0.2.0
- ppv-lite86 0.2.21
- proc-macro2 1.0.106
- quote 1.0.46
- rand 0.10.2
- rand 0.8.7
- rand 0.9.5
- rand_chacha 0.3.1
- rand_chacha 0.9.0
- rand_core 0.10.1
- rand_core 0.6.4
- rand_core 0.9.5
- ref-cast 1.0.25
- ref-cast-impl 1.0.25
- regex 1.13.1
- regex-automata 0.4.16
- regex-syntax 0.8.11
- reqwest 0.13.4
- rsa 0.9.10
- rustc_version 0.4.1
- rustls-pki-types 1.15.0
- scopeguard 1.2.0
- semver 1.0.28
- serde 1.0.228
- serde-untagged 0.1.9
- serde_core 1.0.228
- serde_derive 1.0.228
- serde_derive_internals 0.29.1
- serde_json 1.0.150
- serde_repr 0.1.20
- serde_spanned 1.1.1
- serde_with 3.21.0
- serde_with_macros 3.21.0
- serialize-to-javascript 0.1.2
- serialize-to-javascript-impl 0.1.2
- servo_arc 0.4.3
- sha1 0.10.7
- sha2 0.10.9
- shlex 2.0.1
- signal-hook-registry 1.4.8
- smallvec 1.15.2
- socket2 0.6.5
- stable_deref_trait 1.2.1
- string_cache 0.9.0
- string_cache_codegen 0.6.1
- swift-rs 1.0.7
- syn 2.0.119
- tendril 0.5.1
- thiserror 1.0.69
- thiserror 2.0.18
- thiserror-impl 1.0.69
- thiserror-impl 2.0.18
- thread_local 1.1.10
- time 0.3.53
- time-core 0.1.9
- time-macros 0.2.31
- tokio-rustls 0.26.4
- toml 0.9.12+spec-1.1.0
- toml 1.1.3+spec-1.1.0
- toml_datetime 0.7.5+spec-1.1.0
- toml_datetime 1.1.1+spec-1.1.0
- toml_parser 1.1.2+spec-1.1.0
- toml_writer 1.1.2+spec-1.1.0
- tray-icon 0.24.1
- typeid 1.0.3
- typenum 1.20.1
- unicode-segmentation 1.13.3
- universal-hash 0.5.1
- url 2.5.8
- web-time 1.1.0
- web_atoms 0.2.5

**MIT OR Apache-2.0 OR Zlib** (2)

- raw-window-handle 0.6.2
- tinyvec_macros 0.1.1

**MIT OR BSD-3-Clause** (1)

- if-addrs 0.15.0

**MIT OR Zlib OR Apache-2.0** (1)

- miniz_oxide 0.8.9

**MIT/Apache-2.0** (15)

- bitflags 1.3.2
- bs58 0.5.1
- foreign-types 0.5.0
- foreign-types-macros 0.2.3
- foreign-types-shared 0.3.1
- ident_case 1.0.1
- json-patch 3.0.1
- num-bigint-dig 0.8.6
- siphasher 1.0.3
- unic-char-property 0.9.0
- unic-char-range 0.9.0
- unic-common 0.9.0
- unic-ucd-ident 0.9.0
- unic-ucd-version 0.9.0
- version_check 0.9.5

**MPL-2.0** (5)

- cssparser 0.36.0
- cssparser-macros 0.6.1
- dtoa-short 0.3.5
- option-ext 0.2.0
- selectors 0.36.1

**Unicode-3.0** (18)

- icu_collections 2.2.0
- icu_locale_core 2.2.0
- icu_normalizer 2.2.0
- icu_normalizer_data 2.2.0
- icu_properties 2.2.0
- icu_properties_data 2.2.0
- icu_provider 2.2.0
- litemap 0.8.2
- potential_utf 0.1.5
- tinystr 0.8.3
- writeable 0.6.3
- yoke 0.8.3
- yoke-derive 0.8.2
- zerofrom 0.1.8
- zerofrom-derive 0.1.7
- zerotrie 0.2.4
- zerovec 0.11.6
- zerovec-derive 0.11.3

**Unlicense OR MIT** (3)

- aho-corasick 1.1.4
- byteorder 1.5.0
- memchr 2.8.3

**Unlicense/MIT** (2)

- same-file 1.0.6
- walkdir 2.5.0

**Zlib** (1)

- foldhash 0.2.0

**Zlib OR Apache-2.0 OR MIT** (8)

- dispatch2 0.3.1
- objc2-app-kit 0.3.2
- objc2-core-foundation 0.3.2
- objc2-core-graphics 0.3.2
- objc2-exception-helper 0.1.1
- objc2-io-surface 0.3.2
- objc2-web-kit 0.3.2
- tinyvec 1.12.0


### npm packages bundled into the frontend


**Apache-2.0 OR MIT** (1)

- @tauri-apps/api 2.11.1

**BSD-2-Clause** (1)

- leaflet 1.9.4

**ISC** (1)

- lucide-react 0.468.0

**MIT** (5)

- js-tokens 4.0.0
- loose-envify 1.4.0
- react 18.3.1
- react-dom 18.3.1
- scheduler 0.23.2

**MIT OR Apache-2.0** (1)

- @tauri-apps/plugin-dialog 2.7.1


### Build-time only, not distributed

- @babel/code-frame 7.29.7 (MIT)
- @babel/compat-data 7.29.7 (MIT)
- @babel/core 7.29.7 (MIT)
- @babel/generator 7.29.7 (MIT)
- @babel/helper-compilation-targets 7.29.7 (MIT)
- @babel/helper-globals 7.29.7 (MIT)
- @babel/helper-module-imports 7.29.7 (MIT)
- @babel/helper-module-transforms 7.29.7 (MIT)
- @babel/helper-plugin-utils 7.29.7 (MIT)
- @babel/helper-string-parser 7.29.7 (MIT)
- @babel/helper-validator-identifier 7.29.7 (MIT)
- @babel/helper-validator-option 7.29.7 (MIT)
- @babel/helpers 7.29.7 (MIT)
- @babel/parser 7.29.7 (MIT)
- @babel/plugin-transform-react-jsx-self 7.29.7 (MIT)
- @babel/plugin-transform-react-jsx-source 7.29.7 (MIT)
- @babel/template 7.29.7 (MIT)
- @babel/traverse 7.29.7 (MIT)
- @babel/types 7.29.7 (MIT)
- @esbuild/darwin-arm64 0.25.12 (MIT)
- @jridgewell/gen-mapping 0.3.13 (MIT)
- @jridgewell/remapping 2.3.5 (MIT)
- @jridgewell/resolve-uri 3.1.2 (MIT)
- @jridgewell/sourcemap-codec 1.5.5 (MIT)
- @jridgewell/trace-mapping 0.3.31 (MIT)
- @rolldown/pluginutils 1.0.0-beta.27 (MIT)
- @rollup/rollup-darwin-arm64 4.62.2 (MIT)
- @tauri-apps/cli 2.11.4 (Apache-2.0 OR MIT)
- @tauri-apps/cli-darwin-arm64 2.11.4 (Apache-2.0 OR MIT)
- @types/babel__core 7.20.5 (MIT)
- @types/babel__generator 7.27.0 (MIT)
- @types/babel__template 7.4.4 (MIT)
- @types/babel__traverse 7.28.0 (MIT)
- @types/estree 1.0.9 (MIT)
- @types/geojson 7946.0.16 (MIT)
- @types/leaflet 1.9.21 (MIT)
- @types/prop-types 15.7.15 (MIT)
- @types/react 18.3.31 (MIT)
- @types/react-dom 18.3.7 (MIT)
- @vitejs/plugin-react 4.7.0 (MIT)
- baseline-browser-mapping 2.10.43 (Apache-2.0)
- browserslist 4.28.6 (MIT)
- caniuse-lite 1.0.30001806 (CC-BY-4.0)
- convert-source-map 2.0.0 (MIT)
- csstype 3.2.3 (MIT)
- debug 4.4.3 (MIT)
- electron-to-chromium 1.5.392 (ISC)
- esbuild 0.25.12 (MIT)
- escalade 3.2.0 (MIT)
- fdir 6.5.0 (MIT)
- fsevents 2.3.3 (MIT)
- gensync 1.0.0-beta.2 (MIT)
- jsesc 3.1.0 (MIT)
- json5 2.2.3 (MIT)
- lru-cache 5.1.1 (ISC)
- ms 2.1.3 (MIT)
- nanoid 3.3.16 (MIT)
- node-releases 2.0.51 (MIT)
- picocolors 1.1.1 (ISC)
- picomatch 4.0.5 (MIT)
- postcss 8.5.19 (MIT)
- react-refresh 0.17.0 (MIT)
- rollup 4.62.2 (MIT)
- semver 6.3.1 (ISC)
- source-map-js 1.2.1 (BSD-3-Clause)
- tinyglobby 0.2.17 (MIT)
- typescript 5.7.3 (Apache-2.0)
- update-browserslist-db 1.2.3 (MIT)
- vite 6.4.3 (MIT)
- yallist 3.1.1 (ISC)
