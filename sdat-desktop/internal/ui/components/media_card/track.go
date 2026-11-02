package media_card

import (
	"strings"

	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func NewTrack(track tonearm.Track) schwifty.Button {
	return Card(
		track.Title(),
		SubTitle(strings.Join(track.Artists().Names(), ", ")),
		track.Cover(192),
	).ActionName("win.player.play-track").ActionTargetValue(glib.NewVariantString(track.ID()))
}
