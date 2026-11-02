package player

import (
	"context"
	"math"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var (
	coverState = state.New[schwifty.Paintable](nil)
)

func init() {
	var cancel context.CancelFunc
	player.TrackChanged.On(func(trackInfo *player.TrackChangedEvent) bool {
		if cancel != nil {
			cancel()
			cancel = nil
		}
		if trackInfo != nil {
			cancel = components.RenderCover(trackInfo.Cover(math.MaxInt), coverState)
		} else {
			cancel = components.RenderCover(tonearm.NewURLCover("", ""), coverState)
		}
		return signals.Continue
	})
}

func trackCover() schwifty.Picture {
	return components.SquirclePicture(
		Picture().
			FromPaintable(resources.MissingAlbum()).
			BindPaintable(coverState).HExpand(true).
			HAlign(gtk.AlignCenterValue).
			ConnectConstruct(func(p *gtk.Picture) {
				controller := gtk.NewGestureClick()
				controller.ConnectPressed(new(func(gtk.GestureClick, int32, float64, float64) {
					components.GetMediaViewer().ShowFile(coverState.Value())
				}))
				p.AddController(&controller.EventController)
			}),
	)
}
