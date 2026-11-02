package settings

import (
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type GeneralSettings struct {
	settings *gio.Settings
}

func (g *GeneralSettings) BindRunInBackground(target *gobject.Object, property string) {
	g.settings.Bind("allow-running-in-background", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (g *GeneralSettings) BindDefaultPage(target *gobject.Object, property string) {
	g.settings.Bind("default-page", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (g *GeneralSettings) ShouldHideSecretServiceWarning() bool {
	return g.settings.GetBoolean("hide-secret-service-warning")
}

func (g *GeneralSettings) ShouldRunInBackground() bool {
	return g.settings.GetBoolean("allow-running-in-background")
}

func (g *GeneralSettings) DefaultPage() string {
	return g.settings.GetString("default-page")
}

func (g *GeneralSettings) GetWindowHeight() int32 {
	return g.settings.GetInt("window-height")
}

func (g *GeneralSettings) GetWindowWidth() int32 {
	return g.settings.GetInt("window-width")
}

func (g *GeneralSettings) SetDefaultPage(path string) {
	g.settings.SetString("default-page", path)
}

func (g *GeneralSettings) SetHideSecretServiceWarning(hide bool) bool {
	return g.settings.SetBoolean("hide-secret-service-warning", hide)
}

func (g *GeneralSettings) SetWindowHeight(height int32) {
	g.settings.SetInt("window-height", height)
}

func (g *GeneralSettings) SetWindowWidth(width int32) {
	g.settings.SetInt("window-width", width)
}
