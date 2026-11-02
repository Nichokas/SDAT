package settings

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type ReplayGainMode int32

const (
	// NEVER change the values of existing modes
	ReplayGainModeAuto  ReplayGainMode = 0
	ReplayGainModeAlbum ReplayGainMode = 1
	ReplayGainModeTrack ReplayGainMode = 2
)

func ReplayGainModeStrings() []string {
	return []string{
		gettext.Get("Auto"),
		gettext.Get("Album"),
		gettext.Get("Track"),
	}
}

type PlaybackSettings struct {
	settings *gio.Settings
}

func (p *PlaybackSettings) BindAllowAutoplay(target *gobject.Object, property string) {
	p.settings.Bind("allow-autoplay", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PlaybackSettings) BindNormalizeVolume(target *gobject.Object, property string) {
	p.settings.Bind("normalize-volume", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PlaybackSettings) ConnectNormalizeVolumeChanged(cb func(bool) bool) {
	// Ensure that audio-quality gets watched by gio,
	// since ConnectChanged specifies "
	// 	Note that @settings only emits this signal if you have read
	// 	@key at least once while a signal handler was already connected for @key.
	// "
	if cb(p.NormalizeVolume()) == signals.Unsubscribe {
		return
	}

	var callbackId int
	callbackId = callback.HandleCallback(p.settings.Object, "changed", func(settings gio.Settings, setting string) {
		if setting == "normalize-volume" {
			if cb(p.NormalizeVolume()) == signals.Unsubscribe {
				callback.DeleteCallback(p.settings.Object, "changed", callbackId)
			}
		}
	})
}

func (p *PlaybackSettings) AllowAutoplay() bool {
	return p.settings.GetBoolean("allow-autoplay")
}

func (p *PlaybackSettings) NormalizeVolume() bool {
	return p.settings.GetBoolean("normalize-volume")
}

func (p *PlaybackSettings) ReplayGainMode() ReplayGainMode {
	return ReplayGainMode(p.settings.GetInt("replay-gain-mode"))
}

func (p *PlaybackSettings) SetReplayGainMode(mode ReplayGainMode) {
	p.settings.SetInt("replay-gain-mode", int32(mode))
}
