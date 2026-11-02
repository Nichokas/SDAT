package settings

import (
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type ScrobblingSettings struct {
	settings *gio.Settings
}

func (s *ScrobblingSettings) BindEnableListenBrainz(target *gobject.Object, property string) {
	s.settings.Bind("enable-listenbrainz", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ScrobblingSettings) BindListenBrainzToken(target *gobject.Object, property string) {
	s.settings.Bind("listenbrainz-token", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ScrobblingSettings) BindListenBrainzUrl(target *gobject.Object, property string) {
	s.settings.Bind("listenbrainz-url", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ScrobblingSettings) BindEnableLastFM(target *gobject.Object, property string) {
	s.settings.Bind("enable-lastfm", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ScrobblingSettings) BindLastFMToken(target *gobject.Object, property string) {
	s.settings.Bind("lastfm-token", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ScrobblingSettings) ShouldEnableListenBrainz() bool {
	return s.settings.GetBoolean("enable-listenbrainz")
}
func (s *ScrobblingSettings) ListenBrainzToken() string {
	return s.settings.GetString("listenbrainz-token")
}
func (s *ScrobblingSettings) ListenBrainzUrl() string {
	return s.settings.GetString("listenbrainz-url")
}

func (s *ScrobblingSettings) ShouldEnableLastFM() bool {
	return s.settings.GetBoolean("enable-lastfm")
}
func (s *ScrobblingSettings) LastFMToken() string {
	return s.settings.GetString("lastfm-token")
}

func (s *ScrobblingSettings) SetLastFMToken(token string) {
	s.settings.SetString("lastfm-token", token)
}
