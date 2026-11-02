package shortcut_list

import (
	"strconv"
	"strings"

	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func newAlbum(id string, title string, artists string, coverUrl string) schwifty.Button {
	return NewShortcut(
		title,
		artists,
		coverUrl,
	).ActionName("win.route.album").ActionTargetValue(glib.NewVariantString(id))
}

func NewLegacyAlbum(album *v2.AlbumItemData) schwifty.Button {
	artists := make([]string, 0)
	for _, artist := range album.Artists {
		artists = append(artists, artist.Name)
	}
	return newAlbum(strconv.Itoa(album.Id), album.Title, strings.Join(artists, ", "), tidalapi.ImageURL(album.Cover))
}
