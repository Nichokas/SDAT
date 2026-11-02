package media_card

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/glib"
)

func NewMixGeneric(id string, title string, subtitle string, coverUrl string) schwifty.Button {
	return Card(
		title,
		SubTitle(subtitle).Lines(2),
		tonearm.NewURLCover("", coverUrl),
	).ActionName("win.route.playlist").ActionTargetValue(glib.NewVariantString(id))
}

func NewLegacyMix(mix *v2.MixItemData) schwifty.Button {
	return NewMixGeneric(mix.Id, mix.TitleTextInfo.Text, mix.SubtitleTextInfo.Text, mix.MixImages[0].URL)
}
