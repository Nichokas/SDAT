package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class CoverUrlTest {
    private val base = "https://resources.tidal.com/images/ab/cd"

    @Test fun picksSmallestSizeCoveringThePixels() {
        assertEquals("$base/320x320.jpg", CoverUrl.sized("$base/80x80.jpg", 200))
        assertEquals("$base/1280x1280.jpg", CoverUrl.sized("$base/80x80.jpg", 5000))
        assertEquals("$base/480x480.jpg", CoverUrl.sized("$base/160x160.jpg", 400, CoverUrl.Kind.Artist))
    }

    @Test fun pngIsAlwaysAProfilePicture() {
        assertEquals("$base/320x320.png", CoverUrl.sized("$base/100x100.png", 50))
    }

    @Test fun leavesOtherUrlsAlone() {
        assertEquals("https://example.com/80x80.jpg", CoverUrl.sized("https://example.com/80x80.jpg", 200))
        assertEquals("$base/80x80.jpg", CoverUrl.sized("$base/80x80.jpg", 0))
    }

    @Test fun checksSmallerRenditionsInDescendingOrder() {
        assertEquals(
            listOf("$base/320x320.jpg", "$base/160x160.jpg", "$base/80x80.jpg"),
            CoverUrl.smallerRenditions("$base/640x640.jpg", CoverUrl.Kind.Album),
        )
        assertEquals(
            listOf("$base/480x480.jpg", "$base/320x320.jpg", "$base/160x160.jpg"),
            CoverUrl.smallerRenditions("$base/750x750.jpg", CoverUrl.Kind.Artist),
        )
        assertEquals(emptyList<String>(), CoverUrl.smallerRenditions("$base/320x320.png", CoverUrl.Kind.Artist))
        assertEquals(emptyList<String>(), CoverUrl.smallerRenditions("https://example.com/640x640.jpg", CoverUrl.Kind.Album))
    }
}
