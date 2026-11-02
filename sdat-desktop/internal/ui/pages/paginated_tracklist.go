package pages

import (
	"log/slog"
	"unsafe"

	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func NewPaginatedTracklistPage(
	paginator tonearm.Paginator[tonearm.Track],
	styleFactory func(*tracklist.TrackList) schwifty.BaseWidgetable,
	columns ...tracklist.ColumnFunc,
) (schwifty.ScrolledWindow, error) {
	firstPage, err := paginator.NextPage()
	if err != nil {
		return nil, err
	}

	list := tracklist.NewTrackList(columns...)
	for _, track := range firstPage {
		list.AddTrack(track)
	}

	list.Widget = list.WithCSSClass("page")
	return ScrolledWindow().
		Child(
			components.MainContentScrollable(
				styleFactory(list),
			),
		).
		Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue).
		ConnectReachEdgeSoon(gtk.PosBottomValue, func() bool {
			if !paginator.IsConsumed() {
				items, err := paginator.NextPage()
				if err != nil {
					return signals.Continue
				}

				schwifty.OnMainThreadOnce(func(u uintptr) {
					var list *tracklist.TrackList
					list = (*tracklist.TrackList)(unsafe.Pointer(u))
					for _, track := range items {
						list.AddTrack(track)
					}
				}, uintptr(unsafe.Pointer(list)))
			} else {
				slog.Debug("No more items to fetch")
				return signals.Unsubscribe
			}
			return signals.Continue
		}), nil
}
