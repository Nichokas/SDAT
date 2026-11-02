package queue

import (
	"slices"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/player/playqueue"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var (
	baseQueueState = state.NewStateful([]tonearm.Track{})
	userQueueState = state.NewStateful([]tonearm.Track{})
)

func init() {
	player.BaseQueue.Entries().On(func(tracks []tonearm.Track) bool {
		schwifty.OnMainThreadOnce(func(u uintptr) {
			baseQueueState.SetValue(tracks)
			filledState := queueFilledState.Value()
			filledState.Base = len(tracks) > 0
			queueFilledState.SetValue(filledState)
		}, 0)
		return signals.Continue
	})
	player.UserQueue.Entries().On(func(tracks []tonearm.Track) bool {
		schwifty.OnMainThreadOnce(func(u uintptr) {
			userQueueState.SetValue(tracks)
			filledState := queueFilledState.Value()
			filledState.User = len(tracks) > 0
			queueFilledState.SetValue(filledState)
		}, 0)
		return signals.Continue
	})

}

func makeQueueTracklist(q playqueue.Queue, queueState *state.State[[]tonearm.Track]) *tracklist.TrackList {
	trackList := tracklist.NewTrackList(
		tracklist.CoverColumn, tracklist.TitleAlbumColumn,
		tracklist.CustomWidgetButtonColumn(func(_ string, position int, _ int32) *gtk.Widget {
			return Button().
				TooltipText(gettext.Get("Remove Track from Queue")).
				IconName("user-trash-symbolic").
				WithCSSClass("flat").
				MarginEnd(10).
				ConnectClicked(func(b gtk.Button) {
					go q.RemoveAt(position)
				}).
				ToGTK()
		}),
	)

	trackList.SetClickHandler(func(track tonearm.Track, position int) {
		go player.SkipThroughQueue(q, position)
	})

	trackList.BindTracks(queueState)

	trackList.SetReorderCallback(func(sourceIndex, targetIndex int, track tonearm.Track) {
		q.Entries().Notify(func(oldValue []tonearm.Track) []tonearm.Track {
			q := slices.Clone(oldValue)
			q = append(q[:sourceIndex], q[sourceIndex+1:]...)
			q = append(q[:targetIndex], append([]tonearm.Track{track}, q[targetIndex:]...)...)
			return q
		})
	})

	return trackList
}
