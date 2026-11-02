package settings

import (
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type PerformanceSettings struct {
	settings *gio.Settings
}

func (p *PerformanceSettings) BindAllowMediaCardImages(target *gobject.Object, property string) {
	p.settings.Bind("allow-mediacard-images", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PerformanceSettings) BindAllowShortcutImages(target *gobject.Object, property string) {
	p.settings.Bind("allow-shortcuts-images", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PerformanceSettings) BindAllowTracklistImages(target *gobject.Object, property string) {
	p.settings.Bind("allow-tracklist-images", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PerformanceSettings) BindCacheImages(target *gobject.Object, property string) {
	p.settings.Bind("cache-images", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PerformanceSettings) BindMaxRouterHistorySize(target *gobject.Object, property string) {
	p.settings.Bind("max-router-history-size", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (p *PerformanceSettings) AllowMediaCardImages() bool {
	return p.settings.GetBoolean("allow-mediacard-images")
}

func (p *PerformanceSettings) AllowShortcutImages() bool {
	return p.settings.GetBoolean("allow-shortcuts-images")
}

func (p *PerformanceSettings) AllowTracklistImages() bool {
	return p.settings.GetBoolean("allow-tracklist-images")
}

func (p *PerformanceSettings) MaxRouterHistorySize() int32 {
	return p.settings.GetInt("max-router-history-size")
}

func (p *PerformanceSettings) ShouldCacheImages() bool {
	return p.settings.GetBoolean("cache-images")
}
