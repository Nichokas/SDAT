package media_card

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func NewPlaylist(playlist tonearm.Playlist) schwifty.Button {
	creatorName := "TIDAL"
	if creator := playlist.Creator(); creator != nil {
		creatorName = creator.Title()
	}
	return Card(
		playlist.Title(),
		VStack(
			SubTitle(creatorName),
			SubTitle(gettext.GetN("%d Track", "%d Tracks", playlist.Count(), playlist.Count())),
		),
		playlist.Cover(192),
	).ActionName("win.route.playlist").ActionTargetValue(glib.NewVariantString(playlist.ID()))
}
