package tracklist

import (
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func ButtonColumn(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
	grid.Attach(
		Button().
			ActionName("win.player.play-track").
			ActionTargetValue(glib.NewVariantString(track.ID())).
			WithCSSClass("flat").
			ToGTK(),
		0,
		0,
		column,
		1,
	)
	return 0
}

func ExpandButtonColumn(additionalWidth int32) ColumnFunc {
	return func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
		return ButtonColumn(track, grid, position, column+additionalWidth)
	}
}
