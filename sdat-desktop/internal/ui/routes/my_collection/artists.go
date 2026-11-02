package my_collection

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/pages"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	modelopenapi "codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/pagination"
	"github.com/infinytum/injector"
)

func Artists() *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	if !secrets.HasRefreshToken() {
		return &router.Response{
			PageTitle: gettext.Get("My Collection"),
			View:      components.AuthRequired(gettext.Get("Please sign in to view your collection")),
		}
	}

	paginator := pagination.NewPaginator(tidal.OpenAPI.V2.UserCollections.Artists, "me", func(r *modelopenapi.Response[[]modelopenapi.Relationship]) []modelopenapi.Artist {
		return r.Included.Artists(r.Data...)
	}, "items.profileArt")

	page, err := pages.NewPaginatedMediaCardPage(paginator, func(artist modelopenapi.Artist) schwifty.BaseWidgetable {
		return media_card.NewArtist(openapi.NewArtistInfo(artist))
	})

	return &router.Response{
		PageTitle: gettext.Get("My Artists"),
		Error:     err,
		View:      page,
	}
}
