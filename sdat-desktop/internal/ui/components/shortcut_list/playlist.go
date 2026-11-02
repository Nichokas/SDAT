package shortcut_list

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func newPlaylist(id string, title string, itemCount int, coverUrl string) schwifty.Button {
	return NewShortcut(
		title,
		gettext.GetN("%d Track", "%d Tracks", itemCount, itemCount),
		coverUrl,
	).ActionName("win.route.playlist").ActionTargetValue(glib.NewVariantString(id))
}

func NewLegacyPlaylist(playlist *v2.PlaylistItemData) schwifty.Button {
	return newPlaylist(playlist.UUID, playlist.Title, playlist.NumberOfTracks, tidalapi.ImageURL(playlist.SquareImage))
}
