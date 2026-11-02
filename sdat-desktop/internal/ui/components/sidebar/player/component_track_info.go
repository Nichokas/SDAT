package player

import (
	"strings"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

var titleState = state.NewStateful("")
var artistState = state.NewStateful("")

func init() {
	player.TrackChanged.On(func(trackInfo *player.TrackChangedEvent) bool {
		schwifty.OnMainThreadOncePure(func() {
			if trackInfo == nil {
				titleState.SetValue(gettext.Get("No Track"))
				artistState.SetValue(gettext.Get("No Artist"))
			} else {
				titleState.SetValue(tonearm.FormatTitle(trackInfo))
				artistState.SetValue(strings.Join(trackInfo.Artists().Names(), ", "))
			}
		})
		return signals.Continue
	})
}

func trackInfo() schwifty.Box {
	return VStack(
		Label("").
			BindText(titleState).
			HAlign(gtk.AlignCenterValue).
			Ellipsis(pango.EllipsizeEndValue).
			WithCSSClass("title-4"),
		Label("").
			BindText(artistState).
			HAlign(gtk.AlignCenterValue).
			Ellipsis(pango.EllipsizeEndValue).
			WithCSSClass("heading").WithCSSClass("dimmed"),
	).Spacing(5)
}
