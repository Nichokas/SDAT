package v2

import (
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type Artist struct {
	ArtistInfo
	ArtistPage v2.ArtistPage
}

func (a *Artist) Description() string {
	return a.ArtistPage.Header.Biography.Text
}

func (a Artist) FollowerCount() int {
	return a.ArtistPage.Header.FollowersAmount
}

func NewArtist(artist v2.ArtistPage) tonearm.Artist {
	return &Artist{ArtistInfo{*artist.Item.Data.Artist}, artist}
}
