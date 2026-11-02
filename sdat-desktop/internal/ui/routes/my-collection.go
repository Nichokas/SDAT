package routes

import (
	"context"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/horizontal_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/internal/ui/routes/my_collection"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func init() {
	router.Register("my-collection", MyCollection)
	router.Register("my-collection/albums", my_collection.Albums)
	router.Register("my-collection/artists", my_collection.Artists)
	router.Register("my-collection/playlists", my_collection.Playlists)
	router.Register("my-collection/tracks", my_collection.Tracks)
}

func MyCollection() *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	if !secrets.HasRefreshToken() {
		return &router.Response{
			PageTitle: gettext.Get("My Collection"),
			View:      components.AuthRequired(gettext.Get("Please sign in to view your collection")),
		}
	}

	artistCollection, err := tidal.OpenAPI.V2.UserCollections.Artists(context.Background(), "me", "", "items.profileArt")
	if err != nil {
		return myCollectionErr(err)
	}
	artistList := horizontal_list.NewHorizontalList(gettext.Get("Artists")).SetPageMargin(40).SetViewAllRoute("my-collection/artists")
	for _, artist := range artistCollection.Included.Artists(artistCollection.Data...) {
		artistList.Append(media_card.NewArtist(openapi.NewArtistInfo(artist)))
	}

	albumCollection, err := tidal.OpenAPI.V2.UserCollections.Albums(context.Background(), "me", "", "items.coverArt")
	if err != nil {
		return myCollectionErr(err)
	}
	albumList := horizontal_list.NewHorizontalList(gettext.Get("Albums")).SetPageMargin(40).SetViewAllRoute("my-collection/albums")
	for _, album := range albumCollection.Included.Albums(albumCollection.Data...) {
		albumList.Append(media_card.NewAlbum(openapi.NewAlbum(album)))
	}

	playlistCollection, err := tidal.OpenAPI.V2.UserCollections.Playlists(context.Background(), "me", "", "items.coverArt")
	if err != nil {
		return myCollectionErr(err)
	}
	playlistList := horizontal_list.NewHorizontalList(gettext.Get("Playlists")).SetPageMargin(40).SetViewAllRoute("my-collection/playlists")
	for _, playlist := range playlistCollection.Included.Playlists(playlistCollection.Data...) {
		playlistList.Append(media_card.NewPlaylist(openapi.NewPlaylist(playlist)))
	}

	trackCollection, err := tidal.OpenAPI.V2.UserCollections.Tracks(context.Background(), "me", "", "items.artists", "items.albums.coverArt")
	if err != nil {
		return myCollectionErr(err)
	}
	trackList := tracklist.NewTrackList(
		tracklist.CoverColumn, tracklist.TitleAlbumColumn,
		tracklist.ArtistsColumn,
		tracklist.DurationColumn, tracklist.ControlsColumn,
	)
	for _, track := range trackCollection.Included.Tracks(trackCollection.Data...) {
		trackList.AddTrack(openapi.NewTrack(track))
	}

	return &router.Response{
		PageTitle: gettext.Get("My Collection"),
		View: ScrolledWindow().
			Child(
				components.MainContent(
					VStack(
						artistList,
						albumList,
						playlistList,
						VStack(
							components.NewRowTitle().SetTitle(gettext.Get("Tracks")).SetViewAllRoute("my-collection/tracks"),
							trackList,
						).HMargin(40),
						Spacer(),
					).Spacing(25).VMargin(20).VAlign(gtk.AlignStartValue),
				),
			).
			Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue),
	}
}

func myCollectionErr(err error) *router.Response {
	return &router.Response{
		PageTitle: gettext.Get("My Collection"),
		View:      Label(gettext.Get("Error loading collection")),
		Error:     err,
	}
}
