package com.nichokas.sdat.core

import java.net.URI
import java.util.Base64
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class HlsTest {
    private val base = URI("https://cdn.example/t/1/master.m3u8")

    @Test fun bestVariantPrefersLosslessThenBandwidth() {
        val master = """
            #EXTM3U
            #EXT-X-STREAM-INF:BANDWIDTH=320000,CODECS="mp4a.40.2"
            aac.m3u8
            #EXT-X-STREAM-INF:BANDWIDTH=900000,CODECS="flac"
            flac-low.m3u8
            #EXT-X-STREAM-INF:BANDWIDTH=2500000,CODECS="flac"
            flac-high.m3u8
        """.trimIndent()
        val v = Hls.bestVariant(master, base)!!
        assertEquals(URI("https://cdn.example/t/1/flac-high.m3u8"), v.uri)
        assertEquals(2_500_000, v.bandwidth)
        assertEquals("flac", v.codecs)
        assertNull(Hls.bestVariant("#EXTM3U\n", base))
    }

    @Test fun mediaPlaylistResolvesInitAndSegmentsWithDurations() {
        val media = """
            #EXTM3U
            #EXT-X-TARGETDURATION:4
            #EXT-X-MAP:URI="init.mp4"
            #EXTINF:3.996,
            seg-1.mp4
            #EXTINF:2.5,
            https://other.example/seg-2.mp4
            #EXT-X-ENDLIST
        """.trimIndent()
        val m = Hls.media(media, base)
        assertEquals(URI("https://cdn.example/t/1/init.mp4"), m.map)
        assertEquals(listOf("https://cdn.example/t/1/seg-1.mp4", "https://other.example/seg-2.mp4"), m.segments.map { it.uri.toString() })
        assertEquals(listOf("3.996", "2.5"), m.segments.map { it.duration })
    }

    @Test fun refusesWhatCantBeSaved() {
        val encrypted = "#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://x\"\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4"
        assertEquals(DownloadFailure.encrypted.message, assertThrows(DownloadFailure::class.java) { Hls.media(encrypted, base) }.message)
        val ts = "#EXTINF:4,\ns.ts"
        assertEquals("Unsupported stream format (MPEG-TS).", assertThrows(DownloadFailure::class.java) { Hls.media(ts, base) }.message)
        val ranges = "#EXT-X-MAP:URI=\"i.mp4\",BYTERANGE=\"800@0\"\n#EXTINF:4,\ns.mp4"
        assertThrows(DownloadFailure::class.java) { Hls.media(ranges, base) }
        val twoMaps = "#EXT-X-MAP:URI=\"a.mp4\"\n#EXTINF:4,\ns1.mp4\n#EXT-X-MAP:URI=\"b.mp4\"\n#EXTINF:4,\ns2.mp4"
        assertThrows(DownloadFailure::class.java) { Hls.media(twoMaps, base) }
        val disguised = "#EXT-X-KEY:METHOD=AES-128,URI=\"https://cdn/key?METHOD=NONE\"\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4"
        assertThrows(DownloadFailure::class.java) { Hls.media(disguised, base) }
        // METHOD=NONE is no encryption at all.
        Hls.media("#EXT-X-KEY:METHOD=NONE\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:4,\ns.mp4", base)
    }

    @Test fun inlineManifests() {
        val text = "#EXTM3U\n#EXT-X-MAP:URI=\"https://a/i.mp4\"\n"
        assertEquals(text, Hls.inline("data:application/vnd.apple.mpegurl;base64," + Base64.getEncoder().encodeToString(text.toByteArray())))
        assertEquals("#EXTM3U\n", Hls.inline("data:application/vnd.apple.mpegurl,%23EXTM3U%0A"))
        assertEquals("https://a/b+c d.mp4", Hls.inline("data:application/vnd.apple.mpegurl,https://a/b+c%20d.mp4"))
        assertNull(Hls.inline("https://cdn.example/master.m3u8"))
    }

    @Test fun localPlaylistIndexesTheConcatenatedFile() {
        val media = Hls.media("#EXT-X-MAP:URI=\"i.mp4\"\n#EXTINF:3.996,\ns1.mp4\n#EXT-X-MAP:URI=\"i.mp4\"\n#EXT-X-DISCONTINUITY\n#EXTINF:2.5,\ns2.mp4", base)
        val text = Hls.localPlaylist("1.mp4", 700, media.segments.zip(listOf(1000L, 400L)))
        assertEquals(
            """
            #EXTM3U
            #EXT-X-VERSION:7
            #EXT-X-TARGETDURATION:4
            #EXT-X-PLAYLIST-TYPE:VOD
            #EXT-X-MAP:URI="1.mp4",BYTERANGE="700@0"
            #EXTINF:3.996,
            #EXT-X-BYTERANGE:1000@700
            1.mp4
            #EXT-X-DISCONTINUITY
            #EXTINF:2.5,
            #EXT-X-BYTERANGE:400@1700
            1.mp4
            #EXT-X-ENDLIST

            """.trimIndent(),
            text,
        )
    }

    @Test fun audioSpecFromFlacStreamInfo() {
        val streamInfo = byteArrayOf(0, 16, 0, 16, 0, 0, 0, 0, 0, 0) + bytes(0x17, 0x70, 0x03, 0x70)
        val init = ByteArray(4) + "dfLa".toByteArray() + ByteArray(8) + streamInfo + ByteArray(8)
        assertEquals(96_000 to 24, Hls.audioSpec(init))
    }

    @Test fun audioSpecFromAacSampleEntry() {
        val init = ByteArray(4) + "mp4a".toByteArray() + ByteArray(24) + bytes(0xAC, 0x44, 0, 0)
        assertEquals(44_100 to null, Hls.audioSpec(init))
        assertEquals(null to null, Hls.audioSpec(ByteArray(16)))
    }

    private fun bytes(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }
}
