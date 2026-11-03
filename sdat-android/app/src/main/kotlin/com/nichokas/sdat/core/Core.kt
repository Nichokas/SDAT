package com.nichokas.sdat.core

import uniffi.tonearm_ffi.liveIsInvite

/** Loads libtonearm_ffi.so and runs UniFFI's checksum handshake; false if the Rust core cannot be reached. */
fun coreLoaded(): Boolean = runCatching { liveIsInvite("") }.isSuccess
