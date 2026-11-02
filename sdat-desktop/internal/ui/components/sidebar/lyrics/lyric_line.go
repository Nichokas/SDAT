package lyrics

import (
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func lyricLine(text string, timing *highlightTiming) schwifty.Button {
	var classListener string

	b := Button().
		Child(
			lyricLineText(text),
		).
		HExpand(true).
		PaddingTop(24).
		PaddingBottom(24).
		PaddingStart(16).
		PaddingEnd(16).
		CornerRadius(12)

	if timing != nil {
		b = b.
			WithCSSClass("dimmed").
			ConnectConstruct(func(b *gtk.Button) {
				ptr := b.GoPointer()
				classListener = activeLyricIndex.AddCallback(func(newValue uintptr) {
					widget := gtk.ButtonNewFromInternalPtr(ptr)
					widget.Ref()
					schwifty.OnMainThreadOncePure(func() {
						defer widget.Unref()

						if newValue == ptr {
							widget.RemoveCssClass("dimmed")
						} else {
							widget.AddCssClass("dimmed")
						}
					})
				})
			}).
			ConnectDestroy(func(w gtk.Widget) {
				activeLyricIndex.RemoveCallback(classListener)
			}).
			ConnectClicked(func(gtk.Button) {
				setNewIndex(timing)
				userManuallyScrolled.SetValue(false)
				player.SeekToPosition(timing.Start, true)
			})
	}

	return b
}

func lyricLineText(lyricText string) schwifty.Label {
	return Label(lyricText).
		HAlign(gtk.AlignCenterValue).
		VAlign(gtk.AlignCenterValue).
		WithCSSClass("title-2").
		Wrap(true).
		Justify(gtk.JustifyCenterValue)
}
