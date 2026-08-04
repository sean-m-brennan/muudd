/*
  Copyright 2026 Sean M. Brennan and contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! Links `libautonomous_trust` for the `at-ffi` feature, and does nothing at all
//! without it — the default build must not require a C library to be present,
//! which is what keeps `ethne ⟂ autonomous_trust` true for everyone who is not
//! deploying against a live node.

fn main() {
    println!("cargo:rerun-if-env-changed=AT_LIB_DIR");

    if std::env::var_os("CARGO_FEATURE_AT_FFI").is_none() {
        return;
    }

    let dir = match std::env::var("AT_LIB_DIR") {
        Ok(d) => d,
        Err(_) => {
            // A hard error rather than a link-time mystery: the `at-ffi` feature
            // is only ever enabled deliberately, so an unset path is a mistake
            // worth naming here.
            panic!(
                "the `at-ffi` feature needs AT_LIB_DIR set to the directory holding \
                 libautonomous_trust.so (autonomous_trust's CMake build dir)"
            );
        }
    };

    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-lib=dylib=autonomous_trust");
    // `rustc-link-arg` applies to THIS crate's own link steps — its tests, its
    // examples, its benches — and no further. It is not inherited by a downstream
    // binary that merely depends on `en-at`, so a host built elsewhere in the
    // workspace still gets `error while loading shared libraries:
    // libautonomous_trust.so`. Measured 2026-08-04, correcting the earlier claim
    // here that this covered "the test binaries" generally.
    //
    // A host binary therefore needs one of: `LD_LIBRARY_PATH=$AT_LIB_DIR` at run
    // time, AT installed to a system library path, or its own build script
    // repeating this rpath. Only the last is a real fix, and it belongs to the
    // host, not here.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");

    // `libautonomous_trust.so` carries NO DT_NEEDED for its own dependencies
    // (checked 2026-07-30: only libc/libm/libstdc++/libgcc_s), so every symbol it
    // takes from libsodium, jansson, protobuf-c and OpenSSL is left for the
    // consumer to resolve. AT's own examples never notice, because
    // `add_at_example` links the STATIC library plus `${DEPENDENCIES}` — which
    // leaves the shared library effectively untested as a link target.
    //
    // The right fix is on AT's side (link those PUBLIC so the .so records them);
    // until then a consumer must name them. `AT_LINK_LIBS` overrides the default
    // for a build configured differently — notably `ssl`/`crypto` are only needed
    // when AT was built with `-DAT_ZTA=ON`.
    //
    // `protobuf` (the C++ runtime) is in the list and `protobuf-c` is not enough:
    // AT's `protobuf-c` copy calls into the C++ library, and the linker stops at
    // `undefined reference to google::protobuf::ShutdownProtobufLibrary()`. The
    // comment above named protobuf from the start; the list did not, so the
    // `at-ffi` feature could not link at all until 2026-08-04.
    println!("cargo:rerun-if-env-changed=AT_LINK_LIBS");
    let libs = std::env::var("AT_LINK_LIBS")
        .unwrap_or_else(|_| "sodium jansson protobuf-c protobuf uuid ssl crypto".to_string());
    for lib in libs.split_whitespace() {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
