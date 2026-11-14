package com.nichokas.sdat.core

import java.net.URI
import java.net.URLDecoder
import java.util.Base64

/** Why a track can't be saved; the message is what the UI shows. */
class DownloadFailure(message: String) : Exception(message) {
    companion object {
        val encrypted = DownloadFailure("This track's stream is encrypted (DRM) and can't be saved.")
        fun unsupported(what: String) = DownloadFailure("Unsupported stream format ($what).")
        val cellular = DownloadFailure("Downloading over cellular is off in Settings.")
        val storage = DownloadFailure("Couldn't save the download. Check the free storage.")
    }
}

/** The parts of TIDAL's HLS streams a download needs (iOS Downloads' playlist parsing). */
object Hls {
    data class Variant(val uri: URI, val bandwidth: Int?, val codecs: String?)

    /** [duration] is the EXTINF value as written, so the local playlist repeats it exactly. */
    data class Segment(val uri: URI, val duration: String, val discontinuity: Boolean = false)

    data class Media(val map: URI, val segments: List<Segment>)

    /** A `data:` manifest's text, or null for a URL to fetch. */
    fun inline(url: String): String? {
        if (!url.startsWith("data:")) return null
        val comma = url.indexOf(',').takeIf { it >= 0 } ?: throw DownloadFailure.unsupported("manifest")
        val body = url.substring(comma + 1)
        return if (url.substring(0, comma).endsWith(";base64")) {
            runCatching { String(Base64.getDecoder().decode(body)) }.getOrElse { throw DownloadFailure.unsupported("manifest") }
        } else {
            // Percent-encoding only: a '+' is itself, not a form-encoded space.
            URLDecoder.decode(body.replace("+", "%2B"), Charsets.UTF_8)
        }
    }

    /** Lossless codec first, then the highest BANDWIDTH; null when [text] has no variants. */
    fun bestVariant(text: String, base: URI?): Variant? {
        val lines = lines(text)
        val variants = lines.withIndex().filter { it.value.startsWith("#EXT-X-STREAM-INF:") }.mapNotNull { (i, l) ->
            val uri = lines.drop(i + 1).firstOrNull { it.isNotEmpty() && !it.startsWith("#") } ?: return@mapNotNull null
            Variant(resolve(base, uri) ?: return@mapNotNull null, attr(l, "BANDWIDTH")?.toIntOrNull(), attr(l, "CODECS"))
        }
        return variants.maxWithOrNull(compareBy<Variant> { it.lossless }.thenBy { it.bandwidth ?: 0 })
    }

    fun isMaster(text: String) = "#EXT-X-STREAM-INF" in text

    /** An fMP4 media playlist; encrypted, byte-range, MPEG-TS and multi-init playlists are refused. */
    fun media(text: String, base: URI?): Media {
        var map: URI? = null
        val segments = mutableListOf<Segment>()
        var duration: String? = null
        var discontinuity = false
        for (l in lines(text)) {
            when {
                l.startsWith("#EXT-X-KEY") || l.startsWith("#EXT-X-SESSION-KEY") -> if (attr(l, "METHOD") != "NONE") throw DownloadFailure.encrypted
                l.startsWith("#EXT-X-BYTERANGE") || l.startsWith("#EXT-X-MAP") && "BYTERANGE" in l -> throw DownloadFailure.unsupported("byte ranges")
                l.startsWith("#EXT-X-MAP") -> {
                    val next = attr(l, "URI")?.let { resolve(base, it) }
                    // The saved file holds one init segment, so every fragment must follow the same one.
                    if (map != null && next != map) throw DownloadFailure.unsupported("several init segments")
                    map = next
                }
                l.startsWith("#EXT-X-DISCONTINUITY") && !l.startsWith("#EXT-X-DISCONTINUITY-SEQUENCE") -> discontinuity = true
                l.startsWith("#EXTINF:") -> duration = l.removePrefix("#EXTINF:").substringBefore(',').trim()
                l.isNotEmpty() && !l.startsWith("#") -> {
                    val uri = resolve(base, l) ?: throw DownloadFailure.unsupported("segment URL")
                    segments += Segment(uri, duration ?: throw DownloadFailure.unsupported("segment without duration"), discontinuity)
                    duration = null
                    discontinuity = false
                }
            }
        }
        if (map == null) throw DownloadFailure.unsupported(if (segments.isEmpty()) "empty playlist" else "MPEG-TS")
        if (segments.isEmpty()) throw DownloadFailure.unsupported("empty playlist")
        return Media(map, segments)
    }

    /**
     * A VOD playlist over one file holding the init segment and then every segment, back to back: ExoPlayer seeks a
     * concatenated fragmented MP4 only through such an index.
     */
    fun localPlaylist(file: String, initSize: Long, segments: List<Pair<Segment, Long>>): String = buildString {
        val target = segments.maxOf { it.first.duration.toDouble() }.let { kotlin.math.ceil(it).toLong() }
        append("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:$target\n#EXT-X-PLAYLIST-TYPE:VOD\n")
        append("#EXT-X-MAP:URI=\"$file\",BYTERANGE=\"$initSize@0\"\n")
        var offset = initSize
        for ((segment, size) in segments) {
            if (segment.discontinuity) append("#EXT-X-DISCONTINUITY\n")
            append("#EXTINF:${segment.duration},\n#EXT-X-BYTERANGE:$size@$offset\n$file\n")
            offset += size
        }
        append("#EXT-X-ENDLIST\n")
    }

    /** Sample rate and FLAC bit depth from an fMP4 init segment: the `dfLa` STREAMINFO, else the `mp4a` entry. */
    fun audioSpec(init: ByteArray): Pair<Int?, Int?> {
        fun u(i: Int) = init[i].toInt() and 0xFF
        fun find(tag: String): Int? {
            val t = tag.toByteArray()
            return (4..init.size - 4).firstOrNull { i -> t.indices.all { init[i + it] == t[it] } }
        }
        // dfLa: tag, version/flags, metadata block header, then STREAMINFO's block and frame sizes (10 bytes) before
        // 20 bits of rate, 3 of channels and 5 of bits per sample.
        find("dfLa")?.let { i ->
            val p = i + 4 + 4 + 4 + 10
            if (p + 4 > init.size) return null to null
            val rate = u(p) shl 12 or (u(p + 1) shl 4) or (u(p + 2) shr 4)
            val bits = ((u(p + 2) and 1) shl 4 or (u(p + 3) shr 4)) + 1
            return rate.takeIf { it > 0 } to bits
        }
        // AudioSampleEntry: tag, 6 reserved, data reference, 8 reserved, channels, sample size, 4 more, rate 16.16.
        find("mp4a")?.let { i ->
            val p = i + 4 + 24
            if (p + 2 > init.size) return null to null
            return (u(p) shl 8 or u(p + 1)).takeIf { it > 0 } to null
        }
        return null to null
    }

    private val Variant.lossless get() = codecs?.lowercase()?.let { "flac" in it || "alac" in it } == true

    private fun lines(text: String) = text.lines().map(String::trim)

    // Scanned left to right, so a quoted value is consumed whole and a comma or KEY= inside it is never a boundary.
    private fun attr(line: String, key: String): String? =
        Regex("""([A-Z0-9-]+)=("[^"]*"|[^,]*)""").findAll(line.substringAfter(':'))
            .firstOrNull { it.groupValues[1] == key }?.groupValues?.get(2)?.trim('"')

    private fun resolve(base: URI?, ref: String): URI? = runCatching {
        if (base == null) URI(ref).takeIf { it.isAbsolute } else base.resolve(ref)
    }.getOrNull()
}
