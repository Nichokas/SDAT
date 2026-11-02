package player

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/internal/settings"
	"github.com/go-gst/go-gst/gst"
)

var (
	logger  = slog.With("module", "player")
	playbin *gst.Element
)

func init() {
	gst.Init(nil)

	if pb, err := gst.NewElement("playbin3"); err != nil {
		panic(err)
	} else {
		playbin = pb
	}

	playbin.GetBus().AddWatch(onBusMessage)
	playbin.Connect("notify::volume", onVolumeChange)
	playbin.Connect("about-to-finish", onAboutToFinish)
	playbin.Connect("deep-element-added", onDeepElementAdded)
	initReplayGain()
}

func onAboutToFinish(_ *gst.Element) {
	stateMachine.Fire(statemachine.TRIGGER_GAPLESS_HINT)
}

func onBusMessage(msg *gst.Message) bool {
	switch msg.Type() {
	case gst.MessageError:
		err := msg.ParseError()
		if (err.Code() == gst.ResourceErrorNotAuthorized || err.Code() == gst.ResourceErrorNotFound) && err.Domain() == gst.DomainResource {
			logger.Warn("playback token expired, restarting playback session")
			stateMachine.Fire(statemachine.TRIGGER_TOKEN_EXPIRED)
			return true
		}
		logger.Error("playback failed", "code", err.Code(), "domain", err.Domain(), "message", err.Message(), "error", err.Error(), "debug", err.DebugString())
	case gst.MessageTag:
		onTagsReceived(msg.ParseTags())
	case gst.MessageStreamStart:
		stateMachine.Fire(gst.MessageStreamStart)
	case gst.MessageEOS:
		stateMachine.Fire(gst.MessageEOS)
	}
	return true
}

func onDeepElementAdded(bin, obj any) {
	if element, ok := obj.(*gst.Bin); ok {
		factory := element.GetFactory()
		if factory == nil || factory.GetName() != "decodebin3" {
			return
		}

		element.Connect("pad-added", func(decodebin *gst.Element, pad *gst.Pad) {
			if pad.GetDirection() != gst.PadDirectionSource {
				return
			}
			pad.AddProbe(gst.PadProbeTypeEventDownstream, onPadProbe)
		})
	}
}

func onVolumeChange() {
	if volume, err := playbin.GetProperty("volume"); err != nil {
		return
	} else if volume.(float64) == VolumeChanged.CurrentValue() {
		return
	} else {
		settings.Player().SetVolume(volume.(float64))
		VolumeChanged.Notify(func(oldValue float64) float64 {
			return volume.(float64)
		})
	}
}
