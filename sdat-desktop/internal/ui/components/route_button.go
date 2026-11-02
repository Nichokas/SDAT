package components

import (
	"strings"

	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type RouteButton struct {
	schwifty.BaseWidgetable

	iconState *state.State[string]

	titleState           *state.State[string]
	titleVisibilityState *state.State[bool]
	titleClassState      *state.State[string]

	tooltipState *state.State[string]
}

func (r *RouteButton) Title(title string) *RouteButton {
	r.titleState.SetValue(title)
	r.titleVisibilityState.SetValue(title != "")
	if title == "" {
		r.titleClassState.SetValue("")
	} else {
		r.titleClassState.SetValue("title")
	}
	return r
}

func (r *RouteButton) Icon(iconName string) *RouteButton {
	r.iconState.SetValue(iconName)
	return r
}

func (r *RouteButton) TooltipText(tooltip string) *RouteButton {
	r.tooltipState.SetValue(tooltip)
	return r
}

// NewRouteButton builds a schwifty Button that tells to router to navigate to path on click.
// If root is true, it also tells the router to forget its current history.
func NewRouteButton(path string, root bool) *RouteButton {
	routeButton := &RouteButton{
		iconState: state.NewStateful("image-missing-symbolic"),

		titleState:           state.NewStateful(""),
		titleClassState:      state.NewStateful(""),
		titleVisibilityState: state.NewStateful(false),

		tooltipState: state.NewStateful(""),
	}

	var subscription *signals.Subscription
	routeButton.BaseWidgetable = Button().
		PaddingStart(0).
		PaddingEnd(0).
		MinHeight(24).
		WithCSSClass("flat").
		BindTooltipText(routeButton.tooltipState).
		ConnectConstruct(func(b *gtk.Button) {
			ref := weak.NewWidgetRef(&b.Widget)
			subscription = router.Navigation.On(func(event *router.NavigationEvent) bool {
				schwifty.OnMainThreadOnce(func(u uintptr) {
					ref.Use(func(widget *gtk.Widget) {
						if strings.HasPrefix(event.Path, path) {
							widget.RemoveCssClass("flat")
							widget.AddCssClass("raised")
						} else {
							widget.RemoveCssClass("raised")
							widget.AddCssClass("flat")
						}
					})
				}, 0)
				return signals.Continue
			})
		}).
		ConnectDestroy(func(w gtk.Widget) {
			router.Navigation.Unsubscribe(subscription)
		}).
		ConnectClicked(func(b gtk.Button) {
			if root {
				router.Clear()
			}
			router.Navigate(path)
		}).
		Child(
			HStack(
				Clamp().MaximumSize(16).Child(
					Image().BindIconName(routeButton.iconState),
				),
				Label("").
					BindText(routeButton.titleState).
					BindCSSClass(routeButton.titleClassState).
					BindVisible(routeButton.titleVisibilityState).
					PaddingStart(7).
					PaddingEnd(7),
			).
				Spacing(7).
				HMargin(9).
				VMargin(2),
		)

	return routeButton
}
