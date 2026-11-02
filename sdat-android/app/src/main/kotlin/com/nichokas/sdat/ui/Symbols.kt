package com.nichokas.sdat.ui

/** An iOS SF Symbol name's glyph in SF Pro (private-use codepoint) and its Material Symbols Rounded ligature. */
data class Symbol(val sfCodePoint: Int, val material: String)

/**
 * Every SF Symbol the iOS app uses. Codepoints were matched against SF-Pro.ttf by rendering each glyph;
 * the font itself is never committed. Names ending in `.fill` use the filled Material font.
 */
val symbols: Map<String, Symbol> = mapOf(
    "arrow.clockwise.circle" to Symbol(0x100681, "refresh"),
    "arrow.down" to Symbol(0x100129, "arrow_downward"),
    "arrow.down.circle" to Symbol(0x100078, "arrow_circle_down"),
    "arrow.down.circle.dotted" to Symbol(0x101E5F, "downloading"),
    "arrow.down.circle.fill" to Symbol(0x100079, "download_for_offline"),
    "arrow.up.right" to Symbol(0x10012F, "north_east"),
    "backward.fill" to Symbol(0x10028A, "skip_previous"),
    "camera" to Symbol(0x10031E, "photo_camera"),
    "checkmark" to Symbol(0x100185, "check"),
    "chevron.right" to Symbol(0x10018A, "chevron_right"),
    "chevron.up.chevron.down" to Symbol(0x10018F, "unfold_more"),
    "dot.radiowaves.left.and.right" to Symbol(0x100319, "sensors"),
    "e.square.fill" to Symbol(0x10009D, "explicit"),
    "ellipsis" to Symbol(0x100360, "more_horiz"),
    "exclamationmark.circle" to Symbol(0x10005E, "error"),
    "exclamationmark.triangle.fill" to Symbol(0x1001FF, "warning"),
    "forward.fill" to Symbol(0x10028C, "skip_next"),
    "list.bullet" to Symbol(0x1002F2, "format_list_bulleted"),
    "magnifyingglass" to Symbol(0x1002AB, "search"),
    "minus.circle" to Symbol(0x10004E, "do_not_disturb_on"),
    "music.microphone" to Symbol(0x10046B, "mic"),
    "music.note" to Symbol(0x10046A, "music_note"),
    "music.note.house.fill" to Symbol(0x1004BD, "library_music"),
    "music.note.list" to Symbol(0x10046C, "queue_music"),
    "pause.fill" to Symbol(0x100286, "pause"),
    "play.fill" to Symbol(0x100284, "play_arrow"),
    "plus" to Symbol(0x10017C, "add"),
    "qrcode.viewfinder" to Symbol(0x1003BB, "qr_code_scanner"),
    "repeat.1" to Symbol(0x10029F, "repeat_one"),
    "shuffle" to Symbol(0x10029D, "shuffle"),
    "signpost.right.and.left.fill" to Symbol(0x10176D, "signpost"),
    "square.and.arrow.up" to Symbol(0x100202, "ios_share"),
    "square.stack" to Symbol(0x10040B, "library_books"),
    "star.fill" to Symbol(0x1002C3, "star"),
    "star.slash" to Symbol(0x1002C7, "star_border"),
    "text.badge.plus" to Symbol(0x1002F8, "playlist_add"),
    "text.line.first.and.arrowtriangle.forward" to Symbol(0x10163F, "queue_play_next"),
    "text.line.last.and.arrowtriangle.forward" to Symbol(0x101640, "playlist_play"),
    "trash" to Symbol(0x100211, "delete"),
    "tray" to Symbol(0x100223, "inbox"),
    "wifi.exclamationmark" to Symbol(0x100665, "wifi_off"),
    "xmark.circle" to Symbol(0x100050, "cancel"),
)
