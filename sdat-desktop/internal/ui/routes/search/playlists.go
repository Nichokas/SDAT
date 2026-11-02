package search

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/pages"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	modelopenapi "codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/pagination"
	"github.com/infinytum/injector"
)

func init() {
	router.Register("search/:query/playlists", playlists)
}

func playlists(query string) *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()

	paginator := pagination.NewPaginator(tidal.OpenAPI.V2.SearchResults.Playlists, query, func(r *modelopenapi.Response[[]modelopenapi.Relationship]) []modelopenapi.Playlist {
		return r.Included.Playlists(r.Data...)
	}, "playlists.coverArt")

	page, err := pages.NewPaginatedMediaCardPage(paginator, func(playlist modelopenapi.Playlist) schwifty.BaseWidgetable {
		return media_card.NewPlaylist(openapi.NewPlaylist(playlist))
	})

	return &router.Response{
		PageTitle: gettext.Get("Search"),
		Error:     err,
		View:      page,
	}
}
