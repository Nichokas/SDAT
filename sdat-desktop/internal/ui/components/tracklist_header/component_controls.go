package tracklist_header

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func componentControls(playFunc func(), shuffleFunc func()) schwifty.Box {
	return HStack(
		Button().
			TooltipText(gettext.Get("Shuffle Album")).
			IconName("playlist-shuffle-symbolic").
			WithCSSClass("pill").
			BindSensitive(playerControlsAvailableState).
			ConnectClicked(func(b gtk.Button) {
				shuffleFunc()
			}),
		Button().
			TooltipText(gettext.Get("Play Album")).
			IconName("play-symbolic").
			WithCSSClass("pill").
			WithCSSClass("suggested-action").
			BindSensitive(playerControlsAvailableState).
			ConnectClicked(func(b gtk.Button) {
				playFunc()
			}),
	).Spacing(12).HAlign(gtk.AlignEndValue).HExpand(true)
}
