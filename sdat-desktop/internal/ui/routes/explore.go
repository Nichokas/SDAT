package routes

import (
	"context"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func init() {
	router.Register("explore", ExploreMain)
	router.Register("explore/:page", Explore)
}

func ExploreMain() *router.Response {
	return Explore("explore")
}

func Explore(pageName string) *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	page, err := tidal.V1.Pages.Page(context.Background(), pageName)
	if err != nil {
		return router.FromError(gettext.Get("Explore"), err)
	}

	body := VStack().Spacing(25).VMargin(20)
	for _, row := range page.Rows {
		for _, module := range row.Modules {
			body = body.Append(VStack(
				components.ForModule(module),
			))
		}
	}

	return &router.Response{
		PageTitle: page.Title,
		View: ScrolledWindow().
			Child(
				components.MainContent(
					body,
				),
			).
			Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue),
	}
}
