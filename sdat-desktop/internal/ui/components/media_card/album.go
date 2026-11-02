package media_card

import (
	"strings"

	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func NewAlbum(album tonearm.Album) schwifty.Button {
	return Card(
		album.Title(),
		VStack(
			SubTitle(strings.Join(album.Artists().Names(), ", ")),
			SubTitle(album.ReleasedAt().Format("2006")),
		),
		album.Cover(172),
	).ActionName("win.route.album").ActionTargetValue(glib.NewVariantString(album.ID()))
}
