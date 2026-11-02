package queue

import (
	"log/slog"
	"time"

	"codeberg.org/dergs/tonearm/internal/g"
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components/sidebar"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

const (
	queueScrollMargin = 75
)

type queueFilledType struct {
	User, Base bool
}

func (q queueFilledType) IsFilled() bool {
	return q.User || q.Base
}

var (
	queueFilledState = state.NewStateful[queueFilledType](queueFilledType{})
	queueDisplay     = state.NewStateful[any](StatusPage().Title(gettext.Get("No Tracks in Queue")).IconName("music-queue-empty-symbolic").VExpand(true))
)

var log = slog.With("module", "queue")

func init() {
	queueFilledState.AddCallback(func(newValue queueFilledType) {
		if newValue.IsFilled() {
			queueDisplay.SetValue(queueList())
		} else {
			queueDisplay.SetValue(StatusPage().Title(gettext.Get("No Tracks in Queue")).IconName("music-queue-empty-symbolic").VExpand(true))
		}
	})
}

var queueList = g.Lazy(func() *gtk.ScrolledWindow {
	var (
		trackList     = makeQueueTracklist(player.UserQueue, userQueueState)
		trackListBase = makeQueueTracklist(player.BaseQueue, baseQueueState)
	)

	motionTicker := time.NewTicker(10 * time.Millisecond)

	return ScrolledWindow().
		HMargin(12).
		VExpand(true).
		Child(
			VStack(
				trackList.Background("alpha(var(--view-bg-color), 0.9)").CornerRadius(10).MarginBottom(10),
				trackListBase,
			).VAlign(gtk.AlignStartValue),
		).
		ConnectRealize(func(sw gtk.Widget) {
			ref := weak.NewWidgetRef(&sw)

			action := 0.0

			go func() {
				for {
					select {
					case _, ok := <-motionTicker.C:
						if !ok {
							return
						}

						if action == 0 {
							continue
						}

						ref.Use(func(obj *gtk.Widget) {
							sw := gtk.ScrolledWindowNewFromInternalPtr(obj.Ptr)

							adj := sw.GetVadjustment()
							schwifty.OnMainThreadOnce(func(u uintptr) {
								adj := gtk.AdjustmentNewFromInternalPtr(u)
								defer adj.Unref()

								adj.SetValue(adj.GetValue() + float64(action)*10)
							}, adj.GoPointer())
						})
					}
				}
			}()

			controller := gtk.NewDropControllerMotion()
			controller.ConnectMotion(new(func(controller gtk.DropControllerMotion, _, y float64) {
				ref.Use(func(obj *gtk.Widget) {
					sw := gtk.ScrolledWindowNewFromInternalPtr(obj.Ptr)
					if y < queueScrollMargin {
						speed := (queueScrollMargin - y) / queueScrollMargin
						action = speed * -4
					} else if h := sw.GetAllocatedHeight(); (float64(h) - y) < queueScrollMargin {
						speed := (queueScrollMargin - (float64(h) - y)) / queueScrollMargin
						action = speed * 4
					} else {
						action = 0
					}
				})
			}))

			controller.ConnectLeave(new(func(gtk.DropControllerMotion) {
				action = 0
			}))

			sw.AddController(&controller.EventController)
		})()
})

func NewQueue() schwifty.Box {
	trackLoaded := state.NewStateful(false)

	player.TrackChanged.On(func(t *player.TrackChangedEvent) bool {
		trackLoaded.SetValue(t != nil)
		return signals.Continue
	})

	return VStack(
		sidebar.MiniPlayer().BindVisible(trackLoaded),
		Bin().BindChild(queueDisplay),
	)
}
