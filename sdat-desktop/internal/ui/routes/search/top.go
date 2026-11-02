package search

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/horizontal_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	modelopenapi "codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func TopHits(searchResults *modelopenapi.SearchResult) schwifty.Box {
	artistList := horizontal_list.NewHorizontalList(gettext.Get("Artists")).SetPageMargin(40).SetViewAllRoute("search/" + searchResults.Data[0].ID + "/artists")
	for _, artist := range searchResults.Included.Artists(searchResults.Data[0].Relationships.TopHits.Data...) {
		artistList.Append(media_card.NewArtist(openapi.NewArtistInfo(artist)))
	}

	trackList := tracklist.NewTrackList(
		tracklist.CoverColumn, tracklist.TitleAlbumColumn,
		tracklist.ArtistsColumn,
		tracklist.DurationColumn, tracklist.ControlsColumn,
	)
	for _, track := range searchResults.Included.Tracks(searchResults.Data[0].Relationships.TopHits.Data...) {
		trackList.AddTrack(openapi.NewTrack(track))
	}

	albumList := horizontal_list.NewHorizontalList(gettext.Get("Albums")).SetPageMargin(40).SetViewAllRoute("search/" + searchResults.Data[0].ID + "/albums")
	for _, album := range searchResults.Included.Albums(searchResults.Data[0].Relationships.TopHits.Data...) {
		albumList.Append(media_card.NewAlbum(openapi.NewAlbum(album)))
	}

	playlistList := horizontal_list.NewHorizontalList(gettext.Get("Playlists")).SetPageMargin(40).SetViewAllRoute("search/" + searchResults.Data[0].ID + "/playlists")
	for _, playlist := range searchResults.Included.Playlists(searchResults.Data[0].Relationships.TopHits.Data...) {
		playlistList.Append(media_card.NewPlaylist(openapi.NewPlaylist(playlist)))
	}

	return VStack(
		artistList,
		albumList,
		playlistList,
		VStack(
			components.NewRowTitle().SetTitle(gettext.Get("Tracks")).SetViewAllRoute("search/"+searchResults.Data[0].ID+"/tracks"),
			trackList,
		).HMargin(40),
		Spacer(),
	).Spacing(25).VMargin(20).VAlign(gtk.AlignStartValue)
}
