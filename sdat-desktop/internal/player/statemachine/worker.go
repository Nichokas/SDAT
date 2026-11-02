package statemachine

import (
	"time"

	"codeberg.org/puregotk/puregotk/v4/glib"
	"github.com/go-gst/go-gst/gst"
)

var (
	workerSourceHandle uint32
	workerPlaybin      Playbin
	onUpdateTick       = glib.SourceFunc(func(uintptr) bool {
		PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
			if ok, position := workerPlaybin.QueryPosition(gst.FormatTime); ok {
				state.Position = time.Duration(position)
			}
			return state
		})
		return glib.SOURCE_CONTINUE
	})
)

func startWorker(playbin Playbin) {
	if workerSourceHandle != 0 {
		stopWorker()
	}
	workerPlaybin = playbin
	workerSourceHandle = glib.TimeoutAdd(uint32(250), &onUpdateTick, 0)
}

func stopWorker() {
	if workerSourceHandle == 0 {
		return
	}

	glib.SourceRemove(workerSourceHandle)
	workerPlaybin = nil
	workerSourceHandle = 0
}
