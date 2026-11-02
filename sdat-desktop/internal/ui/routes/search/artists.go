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
	router.Register("search/:query/artists", artists)
}

func artists(query string) *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()

	paginator := pagination.NewPaginator(tidal.OpenAPI.V2.SearchResults.Artists, query, func(r *modelopenapi.Response[[]modelopenapi.Relationship]) []modelopenapi.Artist {
		return r.Included.Artists(r.Data...)
	}, "artists.profileArt")

	page, err := pages.NewPaginatedMediaCardPage(paginator, func(artist modelopenapi.Artist) schwifty.BaseWidgetable {
		return media_card.NewArtist(openapi.NewArtistInfo(artist))
	})

	return &router.Response{
		PageTitle: gettext.Get("Search"),
		Error:     err,
		View:      page,
	}
}
