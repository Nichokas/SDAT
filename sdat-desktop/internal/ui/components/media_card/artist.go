package media_card

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func NewArtist(artist tonearm.ArtistInfo) schwifty.Button {
	return Card(
		artist.Title(),
		HStack(),
		artist.Cover(192),
	).ActionName("win.route.artist").ActionTargetValue(glib.NewVariantString(artist.ID()))
}
