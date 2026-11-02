package player

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var logger = slog.With("module", "ui/components", "component", "player")

var (
	isControllableState = state.NewStateful(false)
)

func init() {
	player.PlaybackStateChanged.On(func(ps player.PlaybackState) bool {
		isControllableState.SetValue(!ps.Loading)
		return signals.Continue
	})
}

func NewPlayer() schwifty.Box {
	return VStack(
		PlayingFrom(),
		trackCover(),
		trackInfo(),
		actionRow(),
		trackTimeline(),
		controls(),
	).
		WithCSSClass("player").
		HPadding(24).
		Spacing(25).
		VAlign(gtk.AlignCenterValue)
}
