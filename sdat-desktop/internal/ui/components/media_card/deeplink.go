package media_card

import (
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func newDeeplink(url string, name string, coverUrl string) schwifty.Button {
	return Card(
		name,
		HStack(),
		tonearm.NewURLCover("", coverUrl),
	).ConnectClicked(func(b gtk.Button) {
		router.Navigate(url)
	})
}

func NewLegacyDeeplink(deeplink *v2.DeepLinkItemData) schwifty.Button {
	var coverUrl string
	if deeplink.Id == "tidal://my-collection/tracks" {
		coverUrl = "https://tidal.com/assets/my-tracks-DTG3pLQW.png"
	}

	return newDeeplink(deeplink.URL, deeplink.Title, coverUrl)
}
