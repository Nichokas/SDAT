package player

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/dergs/tonearm/pkg/utils/imgutil"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
	"github.com/infinytum/injector"
)

var playingFromCoverState = state.NewStateful[schwifty.Paintable](resources.MissingAlbum())
var playingFromTitleState = state.NewStateful[string]("Nothing")
var playingFromCanNavigateState = state.NewStateful[bool](false)
var playingFromNavTargetState = state.NewStateful[string]("")

func init() {
	player.SourceChanged.OnLazy(func(s tonearm.PlaybackSource) bool {
		if s != nil {
			playingFromTitleState.SetValue(s.Title())
			playingFromNavTargetState.SetValue(s.Route())
			playingFromCanNavigateState.SetValue(s.Route() != "")

			cover := s.Cover(80)
			if !cover.HasStatic() {
				schwifty.OnMainThreadOncePure(func() {
					playingFromCoverState.SetValue(resources.MissingAlbum())
				})
				return signals.Continue
			}

			texture, err := injector.MustInject[*imgutil.ImgUtil]().FetchCropped(cover.Static())
			if err != nil {
				slog.Error("failed to load source cover", "error", err)
				return signals.Continue
			}
			schwifty.OnMainThreadOncePure(func() {
				playingFromCoverState.SetValue(texture)
			})
		} else {
			resetPlayingFrom()
		}
		return signals.Continue
	})
}

func resetPlayingFrom() {
	playingFromTitleState.SetValue("Nothing")
	playingFromCoverState.SetValue(resources.MissingAlbum())
	playingFromCanNavigateState.SetValue(false)
	playingFromNavTargetState.SetValue("")
}

func PlayingFrom() schwifty.Button {
	return Button().
		BindSensitive(playingFromCanNavigateState).
		ConnectClicked(func(b gtk.Button) {
			router.Navigate(playingFromNavTargetState.Value())
		}).
		Child(
			HStack(
				VStack(
					Label(gettext.Get("Playing From")).HAlign(gtk.AlignStartValue).WithCSSClass("caption-heading").WithCSSClass("dimmed"),
					Label("").BindText(playingFromTitleState).HAlign(gtk.AlignStartValue).WithCSSClass("heading").Ellipsis(pango.EllipsizeEndValue),
				).VAlign(gtk.AlignCenterValue).MarginEnd(10),
				Image().
					BindPaintable(playingFromCoverState).
					Background("alpha(var(--view-fg-color), 0.1)").
					PixelSize(33).
					HAlign(gtk.AlignEndValue).HExpand(true).
					VAlign(gtk.AlignCenterValue).
					Overflow(gtk.OverflowHiddenValue).CornerRadius(5),
			).HAlign(gtk.AlignFillValue),
		).WithCSSClass("flat").CSS("button { margin-top: -10px; margin-bottom: -10px; margin-left: -10px; margin-right: -10px; padding-top: 10px; padding-bottom: 10px; }")
}
