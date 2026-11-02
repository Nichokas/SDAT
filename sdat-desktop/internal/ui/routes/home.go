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
	router.Register("home", Home)
}

func Home() *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	homeFeed, err := tidal.V2.Home.Feed.Static(context.Background())
	if err != nil {
		return router.FromError(gettext.Get("Home"), err)
	}

	body := VStack().Spacing(25).VMargin(20)
	for _, item := range homeFeed.Items {
		body = body.Append(components.ForPageItem(item))
	}

	return &router.Response{
		PageTitle: gettext.Get("Home"),
		View: ScrolledWindow().
			Child(
				components.MainContent(
					body,
				),
			).
			Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue),
	}
}
