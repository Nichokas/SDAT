package settings

import (
	"codeberg.org/dergs/tonearm/internal/g"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/utils/cutil"
	"codeberg.org/puregotk/puregotk/v4/gio"
)

//go:generate glib-compile-schemas .

var (
	GioSettingsChangedCallback = func(settings gio.Settings, setting string) {
		callback.CallbackHandler[any](settings.Object, "changed", settings, cutil.ParseNullTerminatedString(setting))
	}
)

var General = g.Lazy(func() *GeneralSettings {
	return &GeneralSettings{
		finalize(gio.NewSettings("dev.dergs.Tonearm")),
	}
})

var Playback = g.Lazy(func() *PlaybackSettings {
	settings := gio.NewSettings("dev.dergs.Tonearm.playback")
	settings.ConnectChanged(&GioSettingsChangedCallback)
	tracking.Track(settings.GoPointer(), "Settings")
	return &PlaybackSettings{
		finalize(settings),
	}
})

var Performance = g.Lazy(func() *PerformanceSettings {
	return &PerformanceSettings{
		finalize(gio.NewSettings("dev.dergs.Tonearm.performance")),
	}
})

var Player = g.Lazy(func() *PlayerSettings {
	settings := gio.NewSettings("dev.dergs.Tonearm.player")
	settings.ConnectChanged(&GioSettingsChangedCallback)
	tracking.Track(settings.GoPointer(), "Settings")
	return &PlayerSettings{
		finalize(settings),
	}
})

var Scrobbling = g.Lazy(func() *ScrobblingSettings {
	return &ScrobblingSettings{
		finalize(gio.NewSettings("dev.dergs.Tonearm.scrobbling")),
	}
})

var ServiceTidal = g.Lazy(func() *ServiceTidalSettings {
	settings := gio.NewSettings("dev.dergs.Tonearm.service.tidal")
	settings.ConnectChanged(&GioSettingsChangedCallback)
	tracking.Track(settings.GoPointer(), "Settings")
	return &ServiceTidalSettings{
		finalize(settings),
	}
})

func finalize(settings *gio.Settings) *gio.Settings {
	tracking.SetFinalizer("Settings", settings)
	return settings
}
