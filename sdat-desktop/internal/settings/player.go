package settings

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type PlayerSettings struct {
	settings *gio.Settings
}

func (p *PlayerSettings) ConnectAudioQualityChanged(cb func(tonearm.TrackQuality) bool) {
	// Ensure that audio-quality gets watched by gio,
	// since ConnectChanged specifies "
	// 	Note that @settings only emits this signal if you have read
	// 	@key at least once while a signal handler was already connected for @key.
	// "
	if cb(p.GetAudioQuality()) == signals.Unsubscribe {
		return
	}

	var callbackId int
	callbackId = callback.HandleCallback(p.settings.Object, "changed", func(settings gio.Settings, setting string) {
		if setting == "audio-quality" {
			if cb(p.GetAudioQuality()) == signals.Unsubscribe {
				callback.DeleteCallback(p.settings.Object, "changed", callbackId)
			}
		}
	})
}

func (p *PlayerSettings) GetAudioQuality() tonearm.TrackQuality {
	return p.parseAudioQuality(p.settings.GetString("audio-quality"))
}

func (p *PlayerSettings) SetAudioQuality(quality tonearm.TrackQuality) {
	p.settings.SetString("audio-quality", string(quality))
}

func (p *PlayerSettings) BindVolume(target *gobject.Object, property string) {
	p.settings.Bind("volume", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PlayerSettings) GetVolume() float64 {
	return p.settings.GetDouble("volume")
}

func (p *PlayerSettings) SetVolume(volume float64) {
	p.settings.SetDouble("volume", volume)
}

func (p *PlayerSettings) parseAudioQuality(quality string) tonearm.TrackQuality {
	switch quality {
	case string(tonearm.TrackQualityHighResLossless):
		return tonearm.TrackQualityHighResLossless
	case string(tonearm.TrackQualityLossless):
		return tonearm.TrackQualityLossless
	case string(tonearm.TrackQualityHigh):
		return tonearm.TrackQualityHigh
	case string(tonearm.TrackQualityLossy):
		return tonearm.TrackQualityLossy
	default:
		slog.Error("configured audio quality was invalid, defaulting to maximum", "configured", quality)
		return tonearm.TrackQualityHighResLossless
	}
}
