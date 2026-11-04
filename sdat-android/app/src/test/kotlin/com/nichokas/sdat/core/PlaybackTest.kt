package com.nichokas.sdat.core

import androidx.media3.common.MimeTypes
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlaybackTest {
    @Test fun marksHlsManifestsWithQueryStrings() {
        assertEquals(MimeTypes.APPLICATION_M3U8, mimeType("https://sp.tidal.com/x/master.m3u8?token=abc"))
    }

    @Test fun leavesOtherUrlsToExoPlayer() {
        assertNull(mimeType("https://sp.tidal.com/x/track.flac?m3u8=1"))
        assertNull(mimeType("file:///data/track.flac"))
        assertNull(mimeType("not a url"))
    }
}
